use sea_orm::{Database, DatabaseConnection, TransactionTrait};
use std::sync::Arc;
use tokio::sync::Mutex;

use domain::db_context::DbContextMutexGuard;
use domain::transaction_manager::TransactionManagerMutexGuard;

use crate::db_context::SeaOrmDbContextMutexGuard;

/// SeaORM TransactionManager implementation
pub struct SeaOrmTransactionManagerMutexGuard {
    db: DatabaseConnection,
}

impl SeaOrmTransactionManagerMutexGuard {
    pub async fn new(database_url: &str) -> Result<Self, anyhow::Error> {
        let db = Database::connect(database_url).await?;
        Ok(Self { db })
    }
}

impl TransactionManagerMutexGuard for SeaOrmTransactionManagerMutexGuard {
    type DbContext = SeaOrmDbContextMutexGuard;
    type Error = anyhow::Error;

    fn transaction<T, F, Fut>(
        &self,
        f: F,
    ) -> impl std::future::Future<Output = Result<T, Self::Error>> + Send
    where
        F: FnOnce(Arc<Mutex<Self::DbContext>>) -> Fut + Send,
        Fut: std::future::Future<Output = Result<T, Self::Error>> + Send,
        T: Send,
    {
        let db = self.db.clone();
        async move {
            let txn = db.begin().await?;
            let db_context = SeaOrmDbContextMutexGuard::new(txn);
            let db_context = Arc::new(Mutex::new(db_context));

            match f(db_context.clone()).await {
                Ok(result) => {
                    let mut guard = db_context.lock().await;
                    guard.commit().await?;
                    Ok(result)
                }
                Err(e) => {
                    let mut guard = db_context.lock().await;
                    guard.rollback().await?;
                    Err(e)
                }
            }
        }
    }
}
