use amazon_qldb_driver::QldbDriver;
use anyhow::Result;
use runner::ProgramFlow;
use rusoto_qldb_session::{QldbSession, QldbSessionClient};
use settings::{Environment, ExecuteStatementOpt};
use structopt::StructOpt;
use thiserror::Error;
use tracing_subscriber::{fmt::SubscriberBuilder, EnvFilter};

use crate::runner::Runner;
use crate::settings::{Config, Opt};
use crate::ui::ConsoleUi;
use crate::ui::Ui;

mod command;
mod repl_helper;
mod results;
mod runner;
mod rusoto_driver;
mod settings;
mod transaction;
mod ui;

pub async fn run() -> Result<()> {
    let opt = Opt::from_args();
    configure_tracing(&opt)?;
    let config = match opt.config {
        None => Config::load_default()?,
        Some(ref path) => Config::load(path)?,
    };
    let mut env = Environment::new();
    env.apply_config(&config);
    env.apply_cli(&opt)?;

    //loop {
    rusoto_driver::health_check_start_session(&env).await?;
    let mut runner = Runner::new_with_env(env, &opt.execute).await?;
    if let ProgramFlow::Exit = runner.start().await? {
        return Ok(());
    } else {
        unreachable!() // Restart not yet implemented
    }
    //}
}

fn configure_tracing(opt: &Opt) -> Result<()> {
    let subscriber = SubscriberBuilder::default();

    let level = match opt.verbose {
        0 => "error",
        1 => "info",
        2 => "debug",
        _ => "trace",
    };

    let filter = EnvFilter::from_default_env()
        .add_directive("rustyline=off".parse()?)
        .add_directive(level.parse()?);

    let subscriber = subscriber.with_env_filter(filter);

    if opt.verbose == 3 {
        subscriber.pretty().init()
    } else {
        subscriber.compact().init()
    };

    Ok(())
}

struct Deps<C: QldbSession>
where
    C: QldbSession + Send + Sync + Clone + 'static,
{
    env: Environment,
    driver: QldbDriver<C>,
    ui: Box<dyn Ui>,
}

impl Deps<QldbSessionClient> {
    // Production use: builds a real set of dependencies usign the Rusoto client
    // and ConsoleUi.
    async fn new_with_env(
        env: Environment,
        execute: &Option<ExecuteStatementOpt>,
    ) -> Result<Deps<QldbSessionClient>> {
        let driver = rusoto_driver::build_driver(&env).await?;

        let ui = match execute {
            Some(ref e) => {
                let reader = match e {
                    ExecuteStatementOpt::SingleStatement(statement) => statement,
                    _ => todo!(),
                };
                ConsoleUi::new_for_script(&reader[..], env.clone())?
            }
            None => ConsoleUi::new(env.clone()),
        };

        Ok(Deps {
            env,
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
    async fn new_with<U>(env: Environment, client: C, ui: U) -> Result<Deps<C>>
    where
        U: Ui + 'static,
    {
        use amazon_qldb_driver::{retry, QldbDriverBuilder};

        let driver = QldbDriverBuilder::new()
            .ledger_name(env.ledger().value.clone())
            .transaction_retry_policy(retry::never())
            .build_with_client(client)
            .await?;

        Ok(Deps {
            env,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::testing::*;
    use anyhow::Result;
    use async_trait::async_trait;
    use rusoto_qldb_session::*;

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
            Ok(SendCommandResult {
                start_session: Some(StartSessionResult {
                    session_token: Some(format!("token")),
                    ..Default::default()
                }),
                ..Default::default()
            })
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

        let mut env = Environment::new();
        env.apply_cli(&opt)?;
        let mut runner = Runner {
            deps: Deps::new_with(env, client, ui.clone()).await?,
            current_transaction: None,
        };
        ui.inner().pending.push("help".to_string());
        runner.tick().await?;
        let output = ui.inner().output.pop().unwrap();
        assert_eq!(runner::HELP_TEXT, output);

        Ok(())
    }
}
