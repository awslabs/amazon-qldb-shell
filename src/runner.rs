use anyhow::Result;
use ion_c_sys::reader::IonCReader;
use rusoto_qldb_session::{QldbSession, QldbSessionClient};
use rustyline::error::ReadlineError;

use crate::settings::{Environment, ExecuteStatementOpt};
use crate::transaction::ShellTransaction;
use crate::{Deps, QldbShellError};

pub(crate) const HELP_TEXT: &'static str = r"To start a transaction, enter '\start transaction' or '\begin'. To exit, enter '\exit' or press CTRL-D.";

pub(crate) struct Runner<C>
where
    C: QldbSession + Send + Sync + Clone + 'static,
{
    pub(crate) deps: Deps<C>,
    pub(crate) current_transaction: Option<ShellTransaction>,
}

impl Runner<QldbSessionClient> {
    pub(crate) async fn new_with_env(
        env: Environment,
        execute: &Option<ExecuteStatementOpt>,
    ) -> Result<Runner<QldbSessionClient>> {
        Ok(Runner {
            deps: Deps::new_with_env(env, execute).await?,
            current_transaction: None,
        })
    }
}

fn is_special_command(line: &str) -> bool {
    match &line.to_lowercase()[..] {
        "help" | "quit" | "exit" | "start transaction" | "begin" | "abort" | "commit" => true,
        _ => false
    }
}

impl<C> Runner<C>
where
    C: QldbSession + Send + Sync + Clone + 'static,
{
    pub(crate) async fn start(&mut self) -> Result<()> {
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

    pub(crate) async fn tick(&mut self) -> Result<bool> {
        match self.current_transaction {
            None => self.deps.ui.set_prompt(format!("{} ", self.deps.env.prompt.value)),
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
                        _ if is_special_command(&line) => self.handle_command(&line).await?,
                        _ => match self.current_transaction {
                            Some(_) => self.handle_partiql(&line).await?,
                            None => self.handle_autocommit_partiql(&line).await?,
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

    pub(crate) async fn handle_break(&mut self) -> Result<bool> {
        self.deps.ui.println("CTRL-D");
        Ok(if let Some(_) = self.current_transaction {
            self.handle_abort().await?;
            true
        } else {
            false
        })
    }

    pub(crate) async fn handle_command(&mut self, line: &str) -> Result<bool> {
        match &line.to_lowercase()[..] {
            "help" | "?" => {
                self.deps.ui.println(HELP_TEXT);
            }
            "quit" | "exit" => {
                return Ok(false);
            }
            "start transaction" | "begin" => self.handle_start_transaction(),
            "abort" => self.handle_abort().await?,
            "commit" => self.handle_commit().await?,
            "env" => self.handle_env(),
            "show tables" => self.handle_show_tables().await?,
            _ => Err(QldbShellError::UnknownCommand)?,
        }

        Ok(true)
    }

    pub(crate) fn handle_env(&self) {
        self.deps.ui.println(&format!("{:#?}", self.deps.env));
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
}
