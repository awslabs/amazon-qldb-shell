use amazon_qldb_driver::{ion_compat, transaction::StatementResults, QldbDriverBuilder, QldbError};
use amazon_qldb_driver::{retry, QldbDriver};
use async_trait::async_trait;

use anyhow::Result;
use ion_c_sys::reader::IonCReaderHandle;
use ion_c_sys::result::IonCError;
use itertools::Itertools;
use opt::{ExecuteStatementOpt, FormatMode};
use rusoto_core::{
    credential::{ChainProvider, ProfileProvider, ProvideAwsCredentials},
    Region,
};
use rusoto_qldb_session::{QldbSession, QldbSessionClient};
use std::{str::FromStr, sync::Arc, time::Instant};
use structopt::StructOpt;
use thiserror::Error;
use tokio::{
    sync::{
        mpsc::{channel, Receiver, Sender},
        Mutex,
    },
    task::{self, JoinHandle},
};
#[macro_use]
extern crate log;

use rustyline::error::ReadlineError;

use crate::opt::Opt;
use crate::ui::ConsoleUi;
use crate::ui::Ui;

mod opt;
mod repl_helper;
mod ui;

pub async fn run() -> Result<()> {
    let opt = Opt::from_args();
    configure_logging(&opt)?;
    Runner::new_with_opt(opt)?.start().await
}

fn configure_logging(opt: &Opt) -> Result<(), log::SetLoggerError> {
    let level = match opt.verbose {
        true => log::LevelFilter::Debug,
        false => log::LevelFilter::Info,
    };
    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{}[{}][{}] {}",
                chrono::Local::now().format("[%Y-%m-%d][%H:%M:%S]"),
                record.target(),
                record.level(),
                message
            ))
        })
        .level(level)
        .chain(std::io::stdout())
        .filter(|metadata| metadata.target() != "rustyline")
        .apply()
}

/// Required for static dispatch of [`QldbSessionClient::new_with`].
enum CredentialProvider {
    Profile(ProfileProvider),
    Chain(ChainProvider),
}

#[async_trait]
impl ProvideAwsCredentials for CredentialProvider {
    async fn credentials(
        &self,
    ) -> Result<rusoto_core::credential::AwsCredentials, rusoto_core::credential::CredentialsError>
    {
        use CredentialProvider::*;
        match self {
            Profile(p) => p.credentials().await,
            Chain(c) => c.credentials().await,
        }
    }
}

fn profile_provider(opt: &Opt) -> Result<Option<ProfileProvider>> {
    let it = match &opt.profile {
        Some(p) => {
            let mut prof = ProfileProvider::new()?;
            prof.set_profile(p);
            Some(prof)
        }
        None => None,
    };

    Ok(it)
}

// FIXME: Default region should consider what is set in the Profile.
fn rusoto_region(opt: &Opt) -> Result<Region> {
    let it = match (&opt.region, &opt.qldb_session_endpoint) {
        (Some(r), Some(e)) => Region::Custom {
            name: r.to_owned(),
            endpoint: e.to_owned(),
        },
        (Some(r), None) => match Region::from_str(&r) {
            Ok(it) => it,
            Err(e) => {
                warn!("Unknown region {}: {}. If you know the endpoint, you can specify it and try again.", r, e);
                return Err(e)?;
            }
        },
        (None, Some(e)) => Region::Custom {
            name: Region::default().name().to_owned(),
            endpoint: e.to_owned(),
        },
        (None, None) => Region::default(),
    };

    Ok(it)
}

struct Deps<C: QldbSession>
where
    C: QldbSession + Send + Sync + Clone + 'static,
{
    opt: Opt,
    driver: QldbDriver<C>,
    ui: Box<dyn Ui>,
}

impl Deps<QldbSessionClient> {
    // Production use: builds a real set of dependencies usign the Rusoto client
    // and ConsoleUi.
    fn new_with_opt(opt: Opt) -> Result<Deps<QldbSessionClient>> {
        let provider = profile_provider(&opt)?;
        let region = rusoto_region(&opt)?;
        let creds = match provider {
            Some(p) => CredentialProvider::Profile(p),
            None => CredentialProvider::Chain(ChainProvider::new()),
        };

        // We disable transaction retries because they don't make sense. Users
        // are entering statements, so if the tx fails they actually have to
        // enter them again! We can't simply remember their inputs and try
        // again, as individual statements may be derived from values seen from
        // yet other statements.
        let driver = QldbDriverBuilder::new()
            .ledger_name(&opt.ledger)
            .region(region)
            .credentials_provider(creds)
            .transaction_retry_policy(retry::never())
            .build()?;

        let ui = match opt.execute {
            Some(ref e) => {
                let reader = match e {
                    ExecuteStatementOpt::SingleStatement(statement) => statement,
                    _ => todo!(),
                };
                ConsoleUi::new_for_script(&reader[..])?
            }
            None => ConsoleUi::new(opt.terminator_required),
        };

        Ok(Deps {
            opt,
            driver,
            ui: Box::new(ui),
        })
    }
}

impl<C> Deps<C>
where
    C: QldbSession + Send + Sync + Clone + 'static,
{
    #[cfg(test)]
    fn new_with<U>(opt: Opt, client: C, ui: U) -> Result<Deps<C>>
    where
        U: Ui + 'static,
    {
        let driver = QldbDriverBuilder::new()
            .ledger_name(&opt.ledger)
            .transaction_retry_policy(retry::never())
            .build_with_client(client)?;

        Ok(Deps {
            opt,
            driver,
            ui: Box::new(ui),
        })
    }
}

struct Runner<C>
where
    C: QldbSession + Send + Sync + Clone + 'static,
{
    deps: Option<Deps<C>>,
}

impl Runner<QldbSessionClient> {
    fn new_with_opt(opt: Opt) -> Result<Runner<QldbSessionClient>> {
        Ok(Runner {
            deps: Some(Deps::new_with_opt(opt)?),
        })
    }
}

impl<C> Runner<C>
where
    C: QldbSession + Send + Sync + Clone + 'static,
{
    async fn start(&mut self) -> Result<()> {
        self.deps.as_ref().unwrap().ui.println(
            r#"Welcome to the Amazon QLDB Shell!

To start a transaction type 'start transaction', after which you may enter a series of PartiQL statements.
When your transaction is complete, enter 'commit' or 'abort' as appropriate."#,
        );

        let mut mode = IdleMode::new();
        loop {
            if !self.tick(&mut mode).await? {
                break;
            }
        }
        Ok(())
    }

    async fn tick(&mut self, mode: &mut IdleMode<C>) -> Result<bool> {
        mode.deps.replace(self.deps.take().unwrap());
        let carry_on = mode.tick().await;
        self.deps.replace(mode.deps.take().unwrap());
        carry_on
    }
}

#[derive(Error, Debug)]
enum QldbShellError {
    #[error("usage error: {0}")]
    UsageError(String),
}

const HELP_TEXT: &'static str = "To start a transaction, enter 'start transaction' or 'begin'. To exit, enter 'exit' or press CTRL-D.";

struct IdleMode<C>
where
    C: QldbSession + Send + Sync + Clone + 'static,
{
    deps: Option<Deps<C>>,
    current_transaction: Option<ShellTransaction>,
}

impl<C> IdleMode<C>
where
    C: QldbSession + Send + Sync + Clone + 'static,
{
    fn new() -> IdleMode<C> {
        IdleMode {
            deps: None,
            current_transaction: None,
        }
    }

    fn ui(&mut self) -> &mut Box<dyn Ui> {
        match &mut self.deps {
            Some(deps) => &mut deps.ui,
            None => unreachable!(),
        }
    }

    async fn tick(&mut self) -> Result<bool> {
        match self.current_transaction {
            None => self.ui().set_prompt(format!("qldb> ")),
            Some(_) => self.ui().set_prompt(format!("qldb *> ")),
        }

        let user_input = self.ui().user_input();
        Ok(match user_input {
            Ok(line) => {
                if line.is_empty() {
                    true
                } else {
                    match &line[0..1] {
                        r"\" => self.handle_command(&line[1..]).await?,
                        _ => match self.current_transaction {
                            Some(_) => {
                                self.handle_partiql(&line).await?;
                                true
                            }
                            None => self.handle_command(&line).await?,
                        },
                    }
                }
            }
            Err(ReadlineError::Interrupted) => {
                self.ui().println("CTRL-C");
                true
            }
            Err(ReadlineError::Eof) => {
                self.ui().println("CTRL-D");
                false
            }
            Err(err) => {
                self.ui().warn(&format!("Error: {:?}", err));
                false
            }
        })
    }

    async fn handle_command(&mut self, line: &str) -> Result<bool> {
        match &line.to_lowercase()[..] {
            "help" | "?" => {
                self.ui().println(HELP_TEXT);
            }
            "quit" | "exit" => {
                return Ok(false);
            }
            "start transaction" | "begin" => self.handle_start_transaction(),
            "abort" | "ABORT" => self.handle_abort().await?,
            "commit" | "COMMIT" => self.handle_commit().await?,
            _ => {
                self.ui()
                    .println(r"Unknown command, enter '\help' for a list of commands.");
            }
        }

        Ok(true)
    }

    fn handle_start_transaction(&mut self) {
        if let Some(_) = self.current_transaction {
            self.ui().println("Transaction already open");
            return;
        }

        let new_tx = new_transaction(self.deps.as_ref().unwrap().driver.clone());
        self.current_transaction.replace(new_tx);
    }

    async fn handle_partiql(&mut self, line: &str) -> Result<()> {
        let tx = self
            .current_transaction
            .as_mut()
            .ok_or(QldbShellError::UsageError(format!("No active transaction")))?;

        // TODO: Remove this after fixing deps mess
        let deps = match self.deps {
            Some(ref mut d) => d,
            _ => unreachable!(),
        };
        let Deps { ref ui, opt, .. } = deps;

        let start = Instant::now();

        tx.input
            .send(TransactionRequest::ExecuteStatement(line.to_string()))
            .await?;
        let results = match tx.results.recv().await {
            Some(r) => r?,
            _ => unreachable!(),
        };

        results
            .readers()
            .map(|r| formatted_display(r, &opt.format))
            .intersperse(",\n".to_owned())
            .for_each(|p| ui.print(&p));
        ui.newline();
        let number_of_documents = results.len();
        let noun = match number_of_documents {
            1 => "document",
            _ => "documents",
        };
        let stats = results.execution_stats();
        let server_time = stats.timing_information.processing_time_milliseconds;
        let total_time = Instant::now().duration_since(start).as_millis();
        ui.println(&format!(
            "{} {} in bag (read-ios: {}, server-time: {}ms, total-time: {}ms)",
            number_of_documents, noun, stats.io_usage.read_ios, server_time, total_time
        ));

        Ok(())
    }

    async fn handle_abort(&mut self) -> Result<()> {
        let tx = self
            .current_transaction
            .take()
            .ok_or(QldbShellError::UsageError(format!("No active transaction")))?;

        tx.input.send(TransactionRequest::Abort).await?;
        tx.handle.await?
    }

    async fn handle_commit(&mut self) -> Result<()> {
        let tx = self
            .current_transaction
            .take()
            .ok_or(QldbShellError::UsageError(format!("No active transaction")))?;

        tx.input.send(TransactionRequest::Commit).await?;
        tx.handle.await?
    }
}

struct ShellTransaction {
    input: Sender<TransactionRequest>,
    results: Receiver<Result<StatementResults, QldbError>>,
    handle: JoinHandle<Result<()>>,
}

#[derive(Debug)]
enum TransactionRequest {
    ExecuteStatement(String),
    Commit,
    Abort,
}

fn new_transaction<C>(driver: QldbDriver<C>) -> ShellTransaction
where
    C: QldbSession + Send + Sync + Clone + 'static,
{
    let (input, recv) = channel(1);
    let (output, results) = channel(1);

    let handle = task::spawn(async move {
        let recv = Arc::new(Mutex::new(recv));

        let outcome = driver
            .transact(|mut tx| async {
                loop {
                    let input = async {
                        let mut guard = recv.lock().await;
                        guard.recv().await
                    };

                    match input.await {
                        Some(TransactionRequest::ExecuteStatement(partiql)) => {
                            let results = tx.execute_statement(partiql).await;
                            if let Err(_) = output.send(results).await {
                                panic!("results ch should never be closed");
                            }
                        }
                        Some(TransactionRequest::Commit) => {
                            break tx.ok(()).await;
                        }
                        Some(TransactionRequest::Abort) => {
                            break tx.abort(()).await;
                        }
                        _ => unreachable!(),
                    }
                }
            })
            .await?;

        Ok(outcome)
    });

    ShellTransaction {
        input,
        results,
        handle,
    }
}

fn formatted_display(result: Result<IonCReaderHandle, IonCError>, mode: &FormatMode) -> String {
    let value = match result {
        Ok(v) => v,
        Err(e) => {
            warn!(
                "unable to display document because it could not be parsed: {}",
                e
            );
            return String::new();
        }
    };

    match mode {
        FormatMode::Ion => match ion_compat::to_string_pretty(value) {
            Ok(d) => d,
            Err(e) => {
                warn!("ion formatter is not able to display this document: {}", e);
                return String::new();
            }
        },
        // FormatMode::Json => {
        //     todo!("json is not yet supported");
        // }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::testing::*;
    use anyhow::Result;

    // TODO: Find something better.
    #[derive(Clone)]
    struct TodoClient;

    #[async_trait]
    impl QldbSession for TodoClient {
        async fn send_command(
            &self,
            _input: rusoto_qldb_session::SendCommandRequest,
        ) -> Result<
            rusoto_qldb_session::SendCommandResult,
            rusoto_core::RusotoError<rusoto_qldb_session::SendCommandError>,
        > {
            todo!()
        }
    }

    #[tokio::test]
    async fn start_help() -> Result<()> {
        let opt = Opt {
            ledger: "test".to_string(),
            ..Default::default()
        };

        let client = TodoClient {};
        let ui = TestUi::default();

        let mut runner = Runner {
            deps: Some(Deps::new_with(opt, client, ui.clone())?),
        };
        let mut mode = IdleMode::new();
        ui.inner().pending.push("help".to_string());
        runner.tick(&mut mode).await?;
        let output = ui.inner().output.pop().unwrap();
        assert_eq!(HELP_TEXT, output);

        Ok(())
    }
}
