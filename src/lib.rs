use amazon_qldb_driver::QldbDriver;
use anyhow::Result;
use runner::ProgramFlow;
use rusoto_qldb_session::QldbSession;
use settings::Environment;
use structopt::StructOpt;
use thiserror::Error;

use crate::runner::Runner;
use crate::settings::{Opt, ShellConfig};
use crate::ui::ConsoleUi;
use crate::ui::Ui;

mod command;
pub mod error;
mod repl_helper;
mod results;
mod runner;
mod rusoto_driver;
mod settings;
mod tracing;
mod transaction;
mod ui;

pub async fn run() -> Result<()> {
    let opt = Opt::from_args();
    let verbose = opt.verbose.clone();

    let config = match opt.config {
        None => ShellConfig::load_default()?,
        Some(ref path) => ShellConfig::load(path)?,
    };

    let mut env = Environment::new(config, opt)?;
    let _guard = tracing::configure(verbose, &env)?;

    // Certain properties default differently based on whether stdin is a
    // tty or not. For example, certain messages are suppressed when running
    // `echo ... | qldb`.
    if !atty::is(atty::Stream::Stdin) {
        env.apply_noninteractive_defaults();
    }

    let ui = ConsoleUi::new(env.clone());

    if env.config().ui.display_welcome {
        ui.println(
                r#"Welcome to the Amazon QLDB Shell!

To start a transaction type 'start transaction', after which you may enter a series of PartiQL statements.
When your transaction is complete, enter 'commit' or 'abort' as appropriate."#,
            );
    }

    loop {
        let client = rusoto_driver::health_check_start_session(&env).await?;
        let deps = Deps::new(env.clone(), client, ui.clone()).await?;
        let mut runner = Runner {
            deps,
            current_transaction: None,
        };

        match runner.start().await? {
            ProgramFlow::Exit => return Ok(()),
            ProgramFlow::Restart => {} // loops!
        }
    }
}

struct Deps<C>
where
    C: QldbSession + Send + Sync + Clone + 'static,
{
    env: Environment,
    driver: QldbDriver<C>,
    ui: Box<dyn Ui>,
}

impl<C> Deps<C>
where
    C: QldbSession + Send + Sync + Clone + 'static,
{
    async fn new<U>(env: Environment, client: C, ui: U) -> Result<Deps<C>>
    where
        U: Ui + 'static,
    {
        let driver = rusoto_driver::build_driver(client, env.current_ledger().name.clone()).await?;

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
    use crate::settings::config::ShellConfig;
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
            ledger: Some("test".to_string()),
            ..Default::default()
        };

        let client = TodoClient {};
        let ui = TestUi::default();

        let config = ShellConfig::default();
        let env = Environment::new(config, opt)?;
        let mut runner = Runner {
            deps: Deps::new(env, client, ui.clone()).await?,
            current_transaction: None,
        };
        ui.inner().pending.push("help".to_string());
        runner.tick().await?;
        let output = ui.inner().output.pop().unwrap();
        assert_eq!(runner::HELP_TEXT, output);

        Ok(())
    }
}
