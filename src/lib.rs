use amazon_qldb_driver::{ion_compat, transaction::StatementResults, QldbDriverBuilder, QldbError};
use amazon_qldb_driver::{retry, QldbDriver};
use async_trait::async_trait;

use anyhow::Result;
use ion_c_sys::reader::{IonCReaderHandle, IonCReader};
use ion_c_sys::result::IonCError;
use itertools::Itertools;
use rusoto_core::{
    credential::{ChainProvider, ProfileProvider, ProvideAwsCredentials},
    Region,
};
use rusoto_qldb_session::{QldbSession, QldbSessionClient};
use settings::{Environment, ExecuteStatementOpt, FormatMode};
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

use crate::settings::{AutoCommitMode, Config, Opt};
use crate::ui::ConsoleUi;
use crate::ui::Ui;

mod repl_helper;
mod settings;
mod ui;

pub async fn run() -> Result<()> {
    let opt = Opt::from_args();
    configure_logging(&opt)?;
    let config = Config::load_default()?;
    let mut env = Environment::new();
    env.apply_config(&config);
    env.apply_cli(&opt);
    Runner::new_with_opt(opt, env)?.start().await
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
    env: Environment,
    opt: Opt,
    driver: QldbDriver<C>,
    ui: Box<dyn Ui>,
}

impl Deps<QldbSessionClient> {
    // Production use: builds a real set of dependencies usign the Rusoto client
    // and ConsoleUi.
    fn new_with_opt(opt: Opt, env: Environment) -> Result<Deps<QldbSessionClient>> {
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
            env,
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
    fn new_with<U>(env: Environment, opt: Opt, client: C, ui: U) -> Result<Deps<C>>
    where
        U: Ui + 'static,
    {
        let driver = QldbDriverBuilder::new()
            .ledger_name(&opt.ledger)
            .transaction_retry_policy(retry::never())
            .build_with_client(client)?;

        Ok(Deps {
            env,
            opt,
            driver,
            ui: Box::new(ui),
        })
    }
}

#[derive(Error, Debug)]
enum QldbShellError {
    #[error("usage error: {0}")]
    UsageError(String),

    #[error(r"Unknown command, enter '\help' for a list of commands.")]
    UnknownCommand,
}

const HELP_TEXT: &'static str = r"To start a transaction, enter '\start transaction' or '\begin'. To exit, enter '\exit' or press CTRL-D.";

struct Runner<C>
where
    C: QldbSession + Send + Sync + Clone + 'static,
{
    deps: Deps<C>,
    current_transaction: Option<ShellTransaction>,
}

impl Runner<QldbSessionClient> {
    fn new_with_opt(opt: Opt, env: Environment) -> Result<Runner<QldbSessionClient>> {
        Ok(Runner {
            deps: Deps::new_with_opt(opt, env)?,
            current_transaction: None,
        })
    }
}

impl<C> Runner<C>
where
    C: QldbSession + Send + Sync + Clone + 'static,
{
    async fn start(&mut self) -> Result<()> {
        self.deps.ui.println(
            r#"Welcome to the Amazon QLDB Shell!

To start a transaction type '\start transaction', after which you may enter a series of PartiQL statements.
When your transaction is complete, enter '\commit' or '\abort' as appropriate."#,
        );

        loop {
            match self.tick().await {
                Ok(false) => break,
                Err(e) => self.deps.ui.println(&format!("{}", e)),
                _ => {}
            }
        }
        Ok(())
    }

    async fn tick(&mut self) -> Result<bool> {
        match self.current_transaction {
            None => self.deps.ui.set_prompt(format!("qldb> ")),
            Some(_) => self.deps.ui.set_prompt(format!("qldb *> ")),
        }

        let user_input = self.deps.ui.user_input();
        Ok(match user_input {
            Ok(line) => {
                if line.is_empty() {
                    true
                } else {
                    match &line[0..1] {
                        r"\" => self.handle_command(&line[1..]).await?,
                        _ => match self.current_transaction {
                            Some(_) => self.handle_partiql(&line).await?,
                            None => self.handle_autocommit_partiql_or_command(&line).await?,
                        },
                    }
                }
            }
            Err(ReadlineError::Interrupted) => {
                self.deps.ui.println("CTRL-C");
                true
            }
            Err(ReadlineError::Eof) => self.handle_break().await?,
            Err(err) => {
                self.deps.ui.warn(&format!("Error: {:?}", err));
                false
            }
        })
    }

    async fn handle_break(&mut self) -> Result<bool> {
        self.deps.ui.println("CTRL-D");
        Ok(if let Some(_) = self.current_transaction {
            self.handle_abort().await?;
            true
        } else {
            false
        })
    }

    async fn handle_command(&mut self, line: &str) -> Result<bool> {
        match &line.to_lowercase()[..] {
            "help" | "?" => {
                self.deps.ui.println(HELP_TEXT);
            }
            "quit" | "exit" => {
                return Ok(false);
            }
            "start transaction" | "begin" => self.handle_start_transaction(),
            "abort" | "ABORT" => self.handle_abort().await?,
            "commit" | "COMMIT" => self.handle_commit().await?,
            "env" => self.handle_env(),
            "show-tables" => self.handle_show_tables().await?,
            _ => Err(QldbShellError::UnknownCommand)?,
        }

        Ok(true)
    }

    fn handle_env(&self) {
        self.deps.ui.println(&format!("{:#?}", self.deps.env));
    }

    async fn handle_show_tables(&self) -> Result<()> {
        let table_names = self.deps.driver.transact(|mut tx| async {
            let table_names =
                tx.execute_statement("select VALUE name from information_schema.user_tables where status='ACTIVE'").await?;
            tx.ok(table_names).await
        }).await?;

        for reader in table_names.readers() {
            let mut reader = reader?;
            reader.next()?;
            let name= reader.read_string()?;
            self.deps.ui.println(&format!("- {}", name.as_str()));
        }
        Ok(())
    }

    async fn handle_autocommit_partiql_or_command(&mut self, line: &str) -> Result<bool> {
        match self.handle_command(line).await {
            Err(e) => {
                if let Some(QldbShellError::UnknownCommand) = e.downcast_ref::<QldbShellError>() {
                    self.handle_autocommit_partiql(line).await?;
                    Ok(true)
                } else {
                    Err(e)
                }
            }
            other => other,
        }
    }

    async fn handle_autocommit_partiql(&mut self, line: &str) -> Result<()> {
        if self.deps.opt.auto_commit == AutoCommitMode::Off {
            // We're not in auto-commit mode, but there is no transaction
            return Err(QldbShellError::UsageError(format!(
                "No active transaction and not in auto-commit mode. \
                Start a transaction with '\\start transaction'"
            )))?;
        }
        self.handle_start_transaction();
        if let Err(e) = self.handle_partiql(line).await {
            // By dropping the current transaction, the input channel will be
            // closed which ends the transaction.
            self.current_transaction.take();
            Err(e)?
        }
        self.handle_commit().await
    }

    fn handle_start_transaction(&mut self) {
        if let Some(_) = self.current_transaction {
            self.deps.ui.println("Transaction already open");
            return;
        }

        let new_tx = new_transaction(self.deps.driver.clone());
        self.current_transaction.replace(new_tx);
    }

    async fn handle_partiql(&mut self, line: &str) -> Result<bool> {
        let tx = self
            .current_transaction
            .as_mut()
            .ok_or(QldbShellError::UsageError(format!("No active transaction")))?;

        let start = Instant::now();

        tx.input
            .send(TransactionRequest::ExecuteStatement(line.to_string()))
            .await?;
        let results = match tx.results.recv().await {
            Some(Ok(r)) => r,
            Some(Err(e)) => {
                // Some errors end the transaction, some are recoverable.
                if let QldbError::Rusoto(rusoto_core::RusotoError::Service(ref service)) = e {
                    let broken = match service {
                        rusoto_qldb_session::SendCommandError::BadRequest(_)
                        | rusoto_qldb_session::SendCommandError::InvalidSession(_) => true,
                        _ => false,
                    };
                    if broken {
                        let _ = self.current_transaction.take();
                    }
                }
                Err(e)?
            }
            None => {
                // If the results channel is closed, it means the coroutine has
                // quit. Await it to get the error.
                if let Some(tx) = self.current_transaction.take() {
                    match tx.handle.await? {
                        Ok(()) => unreachable!(),
                        Err(e) => Err(e)?,
                    }
                }

                unreachable!()
            }
        };

        let iter = results
            .readers()
            .map(|r| formatted_display(r, &self.deps.opt.format));
        Itertools::intersperse(iter, ",\n".to_owned()).for_each(|p| self.deps.ui.print(&p));
        self.deps.ui.newline();

        if !self.deps.opt.no_query_metrics {
            let noun = match results.len() {
                1 => "document",
                _ => "documents",
            };
            let stats = results.execution_stats();
            let server_time = stats.timing_information.processing_time_milliseconds;
            let total_time = Instant::now().duration_since(start).as_millis();
            self.deps.ui.println(&format!(
                "{} {} in bag (read-ios: {}, server-time: {}ms, total-time: {}ms)",
                results.len(),
                noun,
                stats.io_usage.read_ios,
                server_time,
                total_time
            ));
        }

        Ok(true)
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
                        Some(TransactionRequest::Abort) | None => {
                            break tx.abort(()).await;
                        }
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
            deps: Deps::new_with(Environment::new(), opt, client, ui.clone())?,
            current_transaction: None,
        };
        ui.inner().pending.push("help".to_string());
        runner.tick().await?;
        let output = ui.inner().output.pop().unwrap();
        assert_eq!(HELP_TEXT, output);

        Ok(())
    }
}
