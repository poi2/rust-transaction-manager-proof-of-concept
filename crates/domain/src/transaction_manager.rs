use std::future::Future;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::db_context::DbContext;

/// Transaction Manager trait
pub trait TransactionManager {
    type DbContext: DbContext;
    type Error: Send + Sync + 'static;

    fn transaction<T, F, Fut>(&self, f: F) -> impl Future<Output = Result<T, Self::Error>> + Send
    where
        F: FnOnce(Arc<Mutex<Self::DbContext>>) -> Fut + Send,
        Fut: Future<Output = Result<T, Self::Error>> + Send,
        T: Send;
}
