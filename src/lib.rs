use amazon_qldb_driver::{QldbDriver, QldbSession};
use anyhow::Result;
use runner::ProgramFlow;
use settings::Environment;
use structopt::StructOpt;
use thiserror::Error;

use crate::runner::Runner;
use crate::settings::{Opt, ShellConfig};
use crate::ui::ConsoleUi;
use crate::ui::Ui;

mod awssdk_driver;
mod command;
mod credentials;
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

- Transactions: By default, the shell implicitly runs each statement in its own transaction and automatically commits the transaction if no errors are found. This is configurable. To start a multi-statement transaction, enter `start transaction` or `begin`.
- PartiQL: QLDB supports a subset of PartiQL, and returns elements of query results in unordered "bags". For more details, see the shell topic in the QLDB Developer Guide.
- Developer Guide: https://docs.aws.amazon.com/qldb/latest/developerguide/data-shell.html
- Reminder: This welcome message can be disabled in the config."#,
            );
    }

    loop {
        let client = awssdk_driver::health_check_start_session(&env).await?;
        let driver = awssdk_driver::build_driver(client, env.current_ledger().name.clone()).await?;
        let deps = Deps {
            env: env.clone(),
            driver,
            ui: Box::new(ui.clone()),
        };

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

#[derive(Error, Debug)]
enum QldbShellError {
    #[error("usage error: {0}")]
    UsageError(String),

    #[error(r"Unknown command, enter 'help' for a list of commands.")]
    UnknownCommand,
}

// FIXME: Make testing support use the core types
// #[cfg(test)]
// mod tests {
//     use super::*;
//     use crate::settings::config::ShellConfig;
//     use crate::ui::testing::*;
//     use amazon_qldb_driver::QldbDriverBuilder;
//     use amazon_qldb_driver_rusoto::testing::TestQldbSessionClient;
//     use anyhow::Result;

//     #[tokio::test]
//     async fn start_help() -> Result<()> {
//         let opt = Opt {
//             ledger: Some("test".to_string()),
//             ..Default::default()
//         };

//         let client = TestQldbSessionClient::default();
//         let driver = QldbDriverBuilder::default()
//             .build_with_client(client)
//             .await?;
//         let ui = TestUi::default();

//         let config = ShellConfig::default();
//         let env = Environment::new(config, opt)?;
//         let mut runner = Runner {
//             deps: Deps::new(env, driver, ui.clone()).await?,
//             current_transaction: None,
//         };
//         ui.inner().pending.push("help".to_string());
//         runner.tick().await?;
//         let output = ui.inner().output.pop().unwrap();
//         assert_eq!(runner::HELP_TEXT, output);

//         Ok(())
//     }
// }
