use amazon_qldb_driver::retry;
use amazon_qldb_driver::{ion_compat, BlockingQldbDriver, QldbDriverBuilder};
use async_trait::async_trait;

use ion_c_sys::reader::IonCReaderHandle;
use ion_c_sys::result::IonCError;
use itertools::Itertools;
use rusoto_core::{
    credential::{ChainProvider, ProfileProvider, ProvideAwsCredentials},
    Region,
};
use std::error::Error as StdError;
use thiserror::Error;

use std::str::FromStr;
#[macro_use]
extern crate log;

use rustyline::error::ReadlineError;

use crate::ui::Ui;
use crate::ui::ConsoleUi;
use structopt::StructOpt;

mod repl_helper;
mod ui;

#[derive(Debug, StructOpt, Default)]
#[structopt(
    name = "qldb-shell",
    about = "A shell for interacting with Amazon QLDB."
)]
struct Opt {
    #[structopt(short, long = "--region")]
    region: Option<String>,

    #[structopt(short, long = "--ledger")]
    ledger: String,

    #[structopt(short = "-s", long = "--qldb-session-endpoint")]
    qldb_session_endpoint: Option<String>,

    #[structopt(short, long = "--profile")]
    profile: Option<String>,

    #[structopt(short, long = "--verbose")]
    verbose: bool,

    #[structopt(short, long = "--format", default_value = "ion")]
    format: FormatMode,

    #[structopt(short, long = "--execute")]
    execute: Option<ExecuteStatementOpt>,
}

#[derive(Debug)]
enum FormatMode {
    Ion,
    // Removing a warning temporarily
    // Json,
}

impl Default for FormatMode {
    fn default() -> Self {
        FormatMode::Ion
    }
}

#[derive(Error, Debug)]
enum ParseFormatModeErr {
    #[error("{0} is not a valid format mode")]
    InvalidFormatMode(String),
}

impl FromStr for FormatMode {
    type Err = ParseFormatModeErr;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match &s.to_lowercase()[..] {
            "ion" | "ion-text" => FormatMode::Ion,
            "json" => todo!("json is not yet supported"),
            _ => return Err(ParseFormatModeErr::InvalidFormatMode(s.into())),
        })
    }
}

#[derive(Debug)]
enum ExecuteStatementOpt {
    SingleStatement(String),
    Stdin,
}

impl FromStr for ExecuteStatementOpt {
    type Err = String; // never happens

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "-" => ExecuteStatementOpt::Stdin,
            _ => ExecuteStatementOpt::SingleStatement(s.into()),
        })
    }
}

pub fn run() -> Result<(), Box<dyn StdError>> {
    let opt = Opt::from_args();
    configure_logging(&opt)?;
    Runner::new(opt)?.start()
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

fn profile_provider(opt: &Opt) -> Result<Option<ProfileProvider>, Box<dyn StdError>> {
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
fn rusoto_region(opt: &Opt) -> Result<Region, Box<dyn StdError>> {
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

struct Deps {
    opt: Opt,
    driver: BlockingQldbDriver,
    ui: Box<dyn Ui>,
}

impl Deps {
    fn new_with_opt(opt: Opt) -> Result<Deps, Box<dyn StdError>> {
        let provider = profile_provider(&opt)?;
        let region = rusoto_region(&opt)?;
        let creds = match provider {
            Some(p) => CredentialProvider::Profile(p),
            None => CredentialProvider::Chain(ChainProvider::new()),
        };

        // We disable transaction retries because they don't make sense. Users are entering statements, so if the tx fails they actually have to enter them again! We can't simply remember
        // their inputs and try again, as individual statements may be derived from values seen from yet other statements.
        let driver = QldbDriverBuilder::new()
            .ledger_name(&opt.ledger)
            .region(region)
            .credentials_provider(creds)
            .transaction_retry_policy(retry::never())
            .build()?
            .into_blocking()?;

        let ui = match opt.execute {
            Some(ref e) => {
                let reader = match e {
                    ExecuteStatementOpt::SingleStatement(statement) => statement,
                    _ => todo!(),
                };
                ConsoleUi::new_for_script(&reader[..])?
            }
            None => ConsoleUi::new(),
        };

        Ok(Deps { opt, driver, ui: Box::new(ui) })
    }
}

struct Runner {
    deps: Option<Deps>,
}

impl Runner {
    fn new(opt: Opt) -> Result<Runner, Box<dyn StdError>> {
        Ok(Runner {
            deps: Some(Deps::new_with_opt(opt)?),
        })
    }

    fn start(&mut self) -> Result<(), Box<dyn StdError>> {
        self.deps.as_ref().unwrap().ui.println(
            r#"Welcome to the Amazon QLDB Shell!

To start a transaction type 'start', after which you may enter a series of PartiQL statements.
When your transaction is complete, enter 'commit' or 'abort' as appropriate."#
        );

        let mut mode = IdleMode::default();
        self.repl(&mut mode)
    }

    fn repl(&mut self, mode: &mut IdleMode) -> Result<(), Box<dyn StdError>>  {
        loop {
            if !self.tick(mode)? {
                break
            }
        }
        Ok(())
    }

    fn tick(&mut self, mode: &mut IdleMode) -> Result<bool, Box<dyn StdError>> {
        mode.deps.replace(self.deps.take().unwrap());
        let carry_on = mode.tick();
        self.deps.replace(mode.deps.take().unwrap());
        carry_on
    }
}

#[derive(Default)]
struct IdleMode {
    deps: Option<Deps>,
}

impl IdleMode {
    fn ui(&mut self) -> &mut Box<dyn Ui> {
        match &mut self.deps {
            Some(deps) => &mut deps.ui,
            None => unreachable!(),
        }
    }

    fn tick(&mut self) -> Result<bool, Box<dyn StdError>> {
        self.ui().set_prompt(format!("qldb> "));
        let user_input = self.ui().user_input();
        Ok(match user_input {
            Ok(line) => {
                match &line[0..1] {
                    r"\" => self.handle_command(&line[1..]),
                    _ => self.handle_command(&line)
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

    fn handle_command(&mut self, line: &str) -> bool {
        match &line.to_lowercase()[..] {
            "help" | "?" => {
                self.ui().println("To start a transaction, enter 'start transaction' or 'begin'. To exit, enter 'exit' or press CTRL-D.");
            }
            "start transaction" | "begin" => {
                let mode = TransactionMode::new(self.deps.take().unwrap());
                let deps = mode.run();
                self.deps.replace(deps);
            }
            "quit" | "exit" => {
                return false;
            }
            _ => {
                self.ui().println(r"Unknown command, enter '\help' for a list of commands.");
            }
        }
        true
    }
}

struct TransactionMode {
    deps: Option<Deps>,
}

impl TransactionMode {
    fn new(deps: Deps) -> TransactionMode {
        TransactionMode { deps: Some(deps) }
    }

    fn run(mut self) -> Deps {
        enum Outcome {
            Commit,
            Abort,
        }

        let deps = self.deps.take().unwrap();
        let Deps { opt, driver, ui } = deps;
        let committed = driver.transact(|mut tx| async {
            ui.set_prompt(format!("qldb(tx: {})> ", tx.id));
            let outcome = loop {
                match ui.user_input() {
                    Ok(line) => {
                        match &line[..] {
                            "help" | "HELP" | "?" => {
                                ui.println("Expecting a series of PartiQL statements or one of 'commit' or 'abort'.");
                            }
                            "abort" | "ABORT" => {
                                break Outcome::Abort;
                            }
                            "commit" | "COMMIT" => {
                                break Outcome::Commit;
                            }
                            partiql => {
                                let results = tx.execute_statement(partiql).await?;

                                results
                                    .readers()
                                    .map(|r| {
                                        formatted_display(r, &opt.format)
                                    })
                                    .intersperse(",\n".to_owned())
                                    .for_each(|p|  ui.print(&p));
                                ui.newline();
                            }
                        }
                    }
                    Err(ReadlineError::Interrupted) => {
                        ui.debug("CTRL-C");
                    }
                    Err(ReadlineError::Eof) => {
                        ui.println("CTRL-D .. aborting");
                        break Outcome::Abort;
                    }
                    Err(err) => {
                        ui.warn(&format!("Error: {:?}", err));
                    }
                }
            };

            match outcome {
                Outcome::Commit => tx.ok(true).await,
                Outcome::Abort => tx.abort(false).await,
            }
        });

        let deps = Deps { opt, driver, ui };

        match committed {
            Ok(true) => deps.ui.println("Transaction committed!"),
            Ok(false) => deps.ui.println("Transaction aborted."),
            Err(e) => {
                deps.ui.println(&format!("Error during transaction: {}", e));
                deps.ui.clear_pending();
            }
        }

        deps
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

    #[test]
    fn test_can_start_the_shell() {
        let opt = Opt {
            ledger: "test".to_string(),
            .. Default::default()
        };
        let mut runner = Runner::new(opt).unwrap();
        let ui = TestUi::default();
        runner.deps.as_mut().unwrap().ui = Box::new(ui);
        let mut mode = IdleMode::default();
        // ui.inner.borrow_mut().pending.push("help".to_string());
        runner.tick(&mut mode).unwrap();
    }
}