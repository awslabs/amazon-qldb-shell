use amazon_qldb_driver::aws_sdk_qldbsession::error::{SendCommandError, SendCommandErrorKind};
use amazon_qldb_driver::aws_sdk_qldbsession::types::SdkError;
use amazon_qldb_driver::{QldbDriver, QldbError, QldbSession, StatementResults};
use anyhow::Result;
use std::{sync::Arc, time::Instant};
use tokio::{
    sync::{
        mpsc::{channel, Receiver, Sender},
        Mutex,
    },
    task::{self, JoinHandle},
};

use crate::QldbShellError;
use crate::{error, runner::Runner};
use crate::{results, runner::TickFlow};

// `handle` is in an Option to allow for partial drops. In the happy case, you
// might want to await it to get some typed result back. However, if the
// transaction goes out of scope, we want to cancel it "quickly". By default,
// JoinHandle tries to "cancel fast" and falls back to "slow". This could weird
// UI artifacts (e.g. if a transaction continues, then fails).
pub(crate) struct ShellTransaction {
    input: Sender<TransactionRequest>,
    results: Receiver<Result<StatementResults, QldbError>>,
    handle: Option<JoinHandle<Result<()>>>,
}

impl Drop for ShellTransaction {
    fn drop(&mut self) {
        if let Some(h) = self.handle.take() {
            h.abort();
        }
    }
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
                        // The `None` variant is actually unreachable. It
                        // *would* signify that the input channel was closed,
                        // however the ch and future are dropped simultaneously
                        // in the case of cancellation.
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
        handle: Some(handle),
    }
}

impl<C> Runner<C>
where
    C: QldbSession + Send + Sync + Clone + 'static,
{
    pub(crate) async fn handle_autocommit_partiql(&mut self, line: &str) -> Result<TickFlow> {
        if !self.deps.env.config().ui.auto_commit {
            // We're not in auto-commit mode, but there is no transaction
            return Err(QldbShellError::UsageError(format!(
                "No active transaction and not in auto-commit mode. \
                Start a transaction with 'start transaction' or 'begin'"
            )))?;
        }
        self.handle_start_transaction()?;
        if let Err(e) = self.handle_partiql(line).await {
            // If we got an error, the transaction might still be open if the
            // error was not fatal to the transaction. So, we should send an
            // abort.
            let _ = self.handle_abort().await; // ignore any error calling abort()
            Err(e)?
        }
        self.handle_commit().await?;
        Ok(TickFlow::Again)
    }

    pub(crate) fn handle_start_transaction(&mut self) -> Result<()> {
        if let Some(_) = self.current_transaction {
            return Err(QldbShellError::UsageError(format!(
                "Transaction already open"
            )))?;
        }

        let new_tx = new_transaction(self.deps.driver.clone());
        self.current_transaction.replace(new_tx);
        Ok(())
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
                if let QldbError::SdkError(SdkError::ServiceError {
                    err: SendCommandError { kind, .. },
                    ..
                }) = &e
                {
                    let broken = match kind {
                        SendCommandErrorKind::InvalidSessionException(_) => true,
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
                if let Some(mut tx) = self.current_transaction.take() {
                    if let Some(h) = tx.handle.take() {
                        let _ = h.await?;
                    }
                }

                unreachable!()
            }
        };

        results::display_results(&results, &self.deps.env.config().ui.format, &self.deps.ui);

        if self.deps.env.config().ui.display_query_metrics {
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
                stats.io_usage.read_i_os,
                server_time,
                total_time
            ));
        }

        Ok(TickFlow::Again)
    }

    pub(crate) async fn handle_abort(&mut self) -> Result<()> {
        let mut tx = self
            .current_transaction
            .take()
            .ok_or(QldbShellError::UsageError(format!("No active transaction")))?;

        tx.input.send(TransactionRequest::Abort).await?;
        if let Some(h) = tx.handle.take() {
            h.await?
        } else {
            Ok(())
        }
    }

    pub(crate) async fn handle_commit(&mut self) -> Result<()> {
        let mut tx = self
            .current_transaction
            .take()
            .ok_or(QldbShellError::UsageError(format!("No active transaction")))?;

        tx.input.send(TransactionRequest::Commit).await?;
        if let Some(h) = tx.handle.take() {
            h.await?
        } else {
            Err(error::bug("transaction committed but there are no results"))?
        }
    }
}
