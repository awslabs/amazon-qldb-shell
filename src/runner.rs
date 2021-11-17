use anyhow::Result;
use core::fmt;
use ion_c_sys::reader::IonCReader;
use rustyline::error::ReadlineError;
use tracing::{instrument, span, trace, Instrument, Level};

use crate::transaction::ShellTransaction;
use crate::{
    command::{self, UseCommand},
    settings::Environment,
};
use crate::{Deps, QldbShellError};

pub(crate) enum ProgramFlow {
    Exit,
    Restart,
}

pub(crate) enum TickFlow {
    Again,
    Restart,
    Exit,
}

pub(crate) const HELP_TEXT: &'static str = r#"Shell Keys
  Enter
    - Runs the statement.
  Escape+Enter (macOS, *nix)
  Shift+Enter (Windows)
    - Starts a new line to enter a statement that spans multiple lines. You can also copy input text with multiple lines and paste it into the shell.
  Ctrl+C
    - Cancels the current command.
  Ctrl+D
    - EOF / exits the current level of the shell. If not in a transaction, exits the shell. If in a transaction, aborts the transaction.

Database commands
  start transaction
  begin
    - Manually starts a transaction. You can run multiple statements within a transaction interactively, or non-interactively by batching commands and statements sequentially. 
    Transactions that are not committed within 30 seconds will time out, and QLDB will reject any changes made during the transaction. 
    For more details and examples, see the QLDB Developer Guide [https://docs.aws.amazon.com/qldb/latest/developerguide/data-shell.html#data-shell-transactions].
  commit
    - Commits a transaction. If there is no transaction in progress, the shell reports an error saying that there is no active transaction.
  abort
    - Aborts a transaction. If there is no transaction in progress, the shell reports an error saying that there is no active transaction.
  help
    - Prints the lists of database and meta commands.
  quit
  exit
    - Quits the shell.

Shell Meta Commands
  \use -l LEDGER_NAME [-p PROFILE] [-r REGION_CODE] [-s QLDB_SESSION_ENDPOINT]
    - Switch to a different ledger (or: region, endpoint, AWS profile) without restarting the shell.
  \set edit-mode [emacs|vi]
    - Toggle between Emacs/Vi keybindings.
  \set terminator-required [true|false] 
    - Toggle if a line terminator is required to end each statement.
  \show tables
    - Display a list of active tables in the current ledger.
  \status
    - Prints out your current region, ledger and Shell version.
  \env
    - Prints out your current environment settings including where they were set from.
"#;

pub(crate) struct Runner {
    pub(crate) deps: Deps,
    pub(crate) current_transaction: Option<ShellTransaction>,
}

impl fmt::Debug for Runner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Runner")
    }
}

fn is_special_command(line: &str) -> bool {
    match &line.to_lowercase()[..] {
        "help" | "quit" | "exit" | "start transaction" | "begin" | "abort" | "commit" => true,
        _ => false,
    }
}

fn build_prompt(env: &Environment, transaction_active: bool) -> String {
    let prompt = match env.config().ui.prompt {
        Some(ref p) => p.clone(),
        _ => "qldb$ACTIVE_TRANSACTION> ".to_string(),
    };
    let current_region = env.current_region();
    let current_ledger = env.current_ledger();

    prompt
        .replace("$REGION", current_region.as_ref())
        .replace("$LEDGER", &current_ledger.name[..])
        .replace(
            "$ACTIVE_TRANSACTION",
            match transaction_active {
                true => " *",
                false => "",
            },
        )
}

impl Runner {
    pub(crate) async fn start(&mut self) -> Result<ProgramFlow> {
        loop {
            let span = span!(Level::TRACE, "tick");
            match self.tick().instrument(span).await {
                Ok(TickFlow::Again) => {}
                Ok(TickFlow::Exit) => return Ok(ProgramFlow::Exit),
                Ok(TickFlow::Restart) => return Ok(ProgramFlow::Restart),
                Err(e) => self.deps.ui.eprintln(&format!("{}", e)),
            }
        }
    }

    #[instrument]
    pub(crate) async fn tick(&mut self) -> Result<TickFlow> {
        self.deps.ui.set_prompt(build_prompt(
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
                    if self.deps.env.config().ui.display_ctrl_signals {
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
        if self.deps.env.config().ui.display_ctrl_signals {
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
            "start transaction" | "begin" => self.handle_start_transaction()?,
            "abort" => self.handle_abort().await?,
            "commit" => self.handle_commit().await?,
            "env" => self.handle_env(),
            "show tables" => self.handle_show_tables().await?,
            "status" => self.handle_status().await?,
            _ => return self.handle_complex_command(line).await,
        }

        Ok(TickFlow::Again)
    }

    pub(crate) async fn handle_complex_command(&mut self, line: &str) -> Result<TickFlow> {
        let iter = line.split_ascii_whitespace();
        let backslash = match command::backslash(iter) {
            Ok(b) => b,
            Err(_) => Err(QldbShellError::UnknownCommand)?,
        };

        match backslash {
            command::Backslash::Set(set) => {
                self.deps.env.update(|env| {
                    match set {
                        command::SetCommand::EditMode(ref mode) => {
                            env.config.ui.edit_mode = mode.clone();
                        }
                        command::SetCommand::TerminatorRequired(ref tf) => {
                            env.config.ui.terminator_required = tf.into();
                        }
                    };
                    Ok(())
                })?;
                self.deps.ui.handle_env_set(&set)?;

                Ok(TickFlow::Again)
            }
            command::Backslash::Use(u) => self.handle_use_command(u).await,
        }
    }

    /// The `use` command lets a user switch ledgers (or: region, endpoint, AWS
    /// profile, etc.) without restarting the shell. [`TickFlow::Restart`] is
    /// used to signal that the current dependencies need to be thrown away and
    /// the outer program loop should restart.
    pub(crate) async fn handle_use_command(&mut self, u: UseCommand) -> Result<TickFlow> {
        self.deps.env.update(|env| {
            if let Some(ledger) = u.ledger {
                env.current_ledger.name = ledger;
            }

            if let Some(region) = u.region {
                env.current_ledger.region = Some(region);
            }

            if let Some(profile) = u.profile {
                env.current_ledger.profile = Some(profile);
            }

            if let Some(url) = u.qldb_session_endpoint {
                env.current_ledger.qldb_session_endpoint = Some(url.to_string());
            }

            Ok(())
        })?;

        let _ = self.deps.env.reload_current_ledger_config().await?;

        Ok(TickFlow::Restart)
    }

    pub(crate) fn handle_env(&self) {
        self.deps.ui.println(&format!("{}", self.deps.env));
    }

    pub(crate) async fn handle_show_tables(&self) -> Result<()> {
        let table_names = self.deps.driver.transact(|mut tx| async {
            let table_names =
                tx.execute_statement("select VALUE name from information_schema.user_tables where status='ACTIVE'").await?.buffered().await?;
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
        // TODO: Return latency information from recent commands if we're able to capture it.
        self.deps.ui.println(&format!(
            "Region: {}, Ledger: {}, Shell version: {}",
            self.deps.env.current_region().as_ref(),
            self.deps.driver.ledger_name(),
            env!("CARGO_PKG_VERSION")
        ));
        Ok(())
    }
}
