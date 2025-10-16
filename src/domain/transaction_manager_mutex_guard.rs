use std::future::Future;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::domain::db_context_mutex_guard::DbContextMutexGuard;

/// 完全にORM非依存なTransactionManager（MutexGuard版）
pub trait TransactionManagerMutexGuard {
    type DbContext: DbContextMutexGuard;
    type Error: Send + Sync + 'static;

    fn transaction<T, F, Fut>(&self, f: F) -> impl Future<Output = Result<T, Self::Error>> + Send
    where
        F: FnOnce(Arc<Mutex<Self::DbContext>>) -> Fut + Send,
        Fut: Future<Output = Result<T, Self::Error>> + Send,
        T: Send;
}