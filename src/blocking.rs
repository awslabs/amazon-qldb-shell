use amazon_qldb_driver::{
    transaction::{TransactionAttempt, TransactionOutcome},
    QldbDriver,
};
use anyhow::Result;
use rusoto_qldb_session::QldbSession;
use std::future::Future;
use tokio::runtime::Runtime;

pub fn into_blocking<C>(async_driver: QldbDriver<C>, runtime: Runtime) -> BlockingQldbDriver<C>
where
    C: QldbSession + Send + Sync + Clone + 'static,
{
    BlockingQldbDriver {
        async_driver,
        runtime,
    }
}

pub struct BlockingQldbDriver<C>
where
    C: QldbSession + Send + Sync + Clone + 'static,
{
    async_driver: QldbDriver<C>,
    runtime: Runtime,
}

impl<C> BlockingQldbDriver<C>
where
    C: QldbSession + Send + Sync + Clone,
{
    pub fn transact<F, Fut, R>(&self, transaction: F) -> Result<R>
    where
        Fut: Future<Output = Result<TransactionOutcome<R>>>,
        F: Fn(TransactionAttempt<C>) -> Fut,
    {
        let fun = &transaction;
        self.runtime
            .block_on(async move { self.async_driver.transact(fun).await })
    }
}
