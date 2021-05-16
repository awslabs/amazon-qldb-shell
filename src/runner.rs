use anyhow::Result;
use core::fmt;
use ion_c_sys::reader::IonCReader;
use rusoto_qldb_session::{QldbSession, QldbSessionClient};
use rustyline::error::ReadlineError;
use tracing::{instrument, span, trace, Instrument, Level};

use crate::{
    command,
    settings::{Environment, ExecuteStatementOpt},
};
use crate::{settings::Setter, transaction::ShellTransaction};
use crate::{Deps, QldbShellError};
use std::time::Instant;

pub(crate) enum ProgramFlow {
    Exit,
    Restart,
}

pub(crate) enum TickFlow {
    Again,
    Restart,
    Exit,
}

pub(crate) const HELP_TEXT: &'static str = r"To start a transaction, enter '\start transaction' or '\begin'. To exit, enter '\exit' or press CTRL-D.";

pub(crate) struct Runner<C>
where
    C: QldbSession + Send + Sync + Clone + 'static,
{
    pub(crate) deps: Deps<C>,
    pub(crate) current_transaction: Option<ShellTransaction>,
}

impl<C> fmt::Debug for Runner<C>
where
    C: QldbSession + Send + Sync + Clone + 'static,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Runner")
    }
}

impl Runner<QldbSessionClient> {
    pub(crate) async fn new(
        client: QldbSessionClient,
        env: Environment,
        execute: &Option<ExecuteStatementOpt>,
    ) -> Result<Runner<QldbSessionClient>> {
        Ok(Runner {
            deps: Deps::new(client, env, execute).await?,
            current_transaction: None,
        })
    }
}

fn is_special_command(line: &str) -> bool {
    match &line.to_lowercase()[..] {
        "help" | "quit" | "exit" | "start transaction" | "begin" | "abort" | "commit" => true,
        _ => false,
    }
}

fn create_prompt(env: &Environment, transaction_active: bool) -> String {
    env.prompt()
        .value
        .replace("$REGION", &env.region().value.name()[..])
        .replace("$LEDGER", &env.ledger().value[..])
        .replace(
            "$ACTIVE_TRANSACTION",
            match transaction_active {
                true => " *",
                false => "",
            },
        )
}

impl<C> Runner<C>
where
    C: QldbSession + Send + Sync + Clone + 'static,
{
    pub(crate) async fn start(&mut self) -> Result<ProgramFlow> {
        if self.deps.env.display_welcome().value {
            self.deps.ui.println(
                r#"Welcome to the Amazon QLDB Shell!

To start a transaction type 'start transaction', after which you may enter a series of PartiQL statements.
When your transaction is complete, enter 'commit' or 'abort' as appropriate."#,
            );
        }

        loop {
            let span = span!(Level::TRACE, "tick");
            match self.tick().instrument(span).await {
                Ok(TickFlow::Again) => {}
                Ok(TickFlow::Exit) => return Ok(ProgramFlow::Exit),
                Ok(TickFlow::Restart) => return Ok(ProgramFlow::Restart),
                Err(e) => self.deps.ui.println(&format!("{}", e)),
            }
        }
    }

    #[instrument]
    pub(crate) async fn tick(&mut self) -> Result<TickFlow> {
        self.deps.ui.set_prompt(create_prompt(
            &self.deps.env,
            self.current_transaction.is_some(),
        ));

        let user_input = self.deps.ui.user_input();
        Ok(match user_input {
            Ok(line) => {
                trace!(line = &line[..], "user input");

                if line.is_empty() {
                    TickFlow::Again
                } else {
                    match &line[0..1] {
                        r"\" => self.handle_command(&line[1..]).await?,
                        _ if is_special_command(&line) => self.handle_command(&line).await?,
                        _ => match self.current_transaction {
                            Some(_) => self.handle_partiql(&line).await?,
                            None => self.handle_autocommit_partiql(&line).await?,
                        },
                    }
                }
            }
            Err(err) => match err.downcast::<ReadlineError>() {
                Ok(ReadlineError::Interrupted) => {
                    if self.deps.env.display_ctrl_signals().value {
                        self.deps.ui.println("CTRL-C");
                    }
                    TickFlow::Again
                }
                Ok(ReadlineError::Eof) => self.handle_break().await?,
                err => {
                    self.deps.ui.warn(&format!("Error: {:?}", err));
                    TickFlow::Exit
                }
            },
        })
    }

    pub(crate) async fn handle_break(&mut self) -> Result<TickFlow> {
        if self.deps.env.display_ctrl_signals().value {
            self.deps.ui.println("CTRL-D");
        }
        Ok(if let Some(_) = self.current_transaction {
            self.handle_abort().await?;
            TickFlow::Again
        } else {
            TickFlow::Exit
        })
    }

    pub(crate) async fn handle_command(&mut self, line: &str) -> Result<TickFlow> {
        match &line.to_lowercase()[..] {
            "help" | "?" => {
                self.deps.ui.println(HELP_TEXT);
            }
            "quit" | "exit" => {
                return Ok(TickFlow::Exit);
            }
            "start transaction" | "begin" => self.handle_start_transaction(),
            "abort" => self.handle_abort().await?,
            "commit" => self.handle_commit().await?,
            "env" => self.handle_env(),
            "show tables" => self.handle_show_tables().await?,
            "status" => self.handle_status().await?,
            "use" => return Ok(TickFlow::Restart), // TODO: implement
            _ => self.handle_complex_command(line)?,
        }

        Ok(TickFlow::Again)
    }

    pub(crate) fn handle_complex_command(&mut self, line: &str) -> Result<()> {
        let iter = line.split_ascii_whitespace();
        let backslash = match command::backslash(iter) {
            Ok(b) => b,
            Err(_) => Err(QldbShellError::UnknownCommand)?,
        };

        if let command::Backslash::Set(set) = backslash {
            // FIXME: Hack
            let mut inner = self.deps.env.inner.lock().unwrap();
            match set {
                command::SetCommand::EditMode(ref mode) => {
                    inner.edit_mode.apply_value(mode, Setter::Environment)
                }
                command::SetCommand::TerminatorRequired(ref tf) => {
                    inner.terminator_required.apply_value(
                        &match tf {
                            command::TrueFalse::True => true,
                            command::TrueFalse::False => false,
                        },
                        Setter::Environment,
                    )
                }
            }
            drop(inner); // FIXME: end hack
            self.deps.ui.handle_env_set(&set)?;
        }

        Ok(())
    }

    pub(crate) fn handle_env(&self) {
        self.deps.ui.println(&format!("{}", self.deps.env));
    }

    pub(crate) async fn handle_show_tables(&self) -> Result<()> {
        let table_names = self.deps.driver.transact(|mut tx| async {
            let table_names =
                tx.execute_statement("select VALUE name from information_schema.user_tables where status='ACTIVE'").await?;
            tx.commit(table_names).await
        }).await?;

        for reader in table_names.readers() {
            let mut reader = reader?;
            reader.next()?;
            let name = reader.read_string()?;
            self.deps.ui.println(&format!("- {}", name.as_str()));
        }
        Ok(())
    }

    pub(crate) async fn handle_status(&self) -> Result<()> {
        let start = Instant::now();

        let result = self.deps.driver.transact(|mut tx| async {
            let result = tx
                .execute_statement("select VALUE name from information_schema.user_tables where status='ACTIVE'")
                .await?;
            tx.commit(result).await
        }).await;

        match result {
            Ok(statement_results) => {
                let stats = statement_results.execution_stats();
                let server_time = stats.timing_information.processing_time_milliseconds;
                let total_time = Instant::now().duration_since(start).as_millis();
                self.deps.ui.println(&format!(
                    "Connection status: Connected, server-time: {}ms, roundtrip-time: {}ms",
                    server_time,
                    total_time
                ));
            }
            Err(_e) => {
                self.deps.ui.println(&format!("Connection status: Unavailable"));
            }
        }

        self.deps.ui.println(&format!("Current Ledger: {}", self.deps.driver.ledger_name()));
        self.deps.ui.println(&format!("Client version: {}", env!("CARGO_PKG_VERSION")));
        Ok(())
    }
}
