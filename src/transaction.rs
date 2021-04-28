use amazon_qldb_driver::QldbDriver;
use amazon_qldb_driver::{transaction::StatementResults, QldbError};
use anyhow::Result;
use rusoto_qldb_session::QldbSession;
use std::{sync::Arc, time::Instant};
use tokio::{
    sync::{
        mpsc::{channel, Receiver, Sender},
        Mutex,
    },
    task::{self, JoinHandle},
};

use crate::runner::Runner;
use crate::QldbShellError;
use crate::{results, runner::TickFlow};

pub(crate) struct ShellTransaction {
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
                            break tx.commit(()).await;
                        }
                        Some(TransactionRequest::Abort) | None => {
                            break tx.abort().await;
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

impl<C> Runner<C>
where
    C: QldbSession + Send + Sync + Clone + 'static,
{
    pub(crate) async fn handle_autocommit_partiql(&mut self, line: &str) -> Result<TickFlow> {
        if !self.deps.env.auto_commit.value {
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
        self.handle_commit().await?;
        Ok(TickFlow::Again)
    }

    pub(crate) fn handle_start_transaction(&mut self) {
        if let Some(_) = self.current_transaction {
            self.deps.ui.println("Transaction already open");
            return;
        }

        let new_tx = new_transaction(self.deps.driver.clone());
        self.current_transaction.replace(new_tx);
    }

    pub(crate) async fn handle_partiql(&mut self, line: &str) -> Result<TickFlow> {
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

        results::display_results(&results, &self.deps.env.format.value, &self.deps.ui);

        if self.deps.env.show_query_metrics.value {
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

        Ok(TickFlow::Again)
    }

    pub(crate) async fn handle_abort(&mut self) -> Result<()> {
        let tx = self
            .current_transaction
            .take()
            .ok_or(QldbShellError::UsageError(format!("No active transaction")))?;

        tx.input.send(TransactionRequest::Abort).await?;
        tx.handle.await?
    }

    pub(crate) async fn handle_commit(&mut self) -> Result<()> {
        let tx = self
            .current_transaction
            .take()
            .ok_or(QldbShellError::UsageError(format!("No active transaction")))?;

        tx.input.send(TransactionRequest::Commit).await?;
        tx.handle.await?
    }
}
