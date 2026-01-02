use std::sync::Arc;

use domain::{db_context::DbContext, transaction_manager::TransactionManager};
use sea_orm::{Database, DatabaseConnection, TransactionTrait};
use tokio::sync::Mutex;

use crate::db_context::SeaOrmDbContext;

/// SeaORM TransactionManager implementation
pub struct SeaOrmTransactionManager {
    db: DatabaseConnection,
}

impl SeaOrmTransactionManager {
    pub async fn new(database_url: &str) -> Result<Self, anyhow::Error> {
        let db = Database::connect(database_url).await?;
        Ok(Self { db })
    }
}

impl TransactionManager for SeaOrmTransactionManager {
    type DbContext = SeaOrmDbContext;
    type Error = anyhow::Error;

    async fn transaction<T, F, Fut>(&self, f: F) -> Result<T, Self::Error>
    where
        F: FnOnce(Arc<Mutex<Self::DbContext>>) -> Fut + Send,
        Fut: std::future::Future<Output = Result<T, Self::Error>> + Send,
        T: Send,
    {
        let db = self.db.clone();
        let txn = db.begin().await?;
        let db_context = SeaOrmDbContext::new(txn);
        let db_context = Arc::new(Mutex::new(db_context));

        match f(db_context.clone()).await {
            Ok(result) => match Arc::try_unwrap(db_context) {
                Ok(mutex) => {
                    let context = mutex.into_inner();
                    context.commit().await?;
                    Ok(result)
                }
                Err(_) => Err(anyhow::anyhow!("Failed to extract context for commit")),
            },
            Err(e) => {
                match Arc::try_unwrap(db_context) {
                    Ok(mutex) => {
                        let context = mutex.into_inner();
                        let _ = context.rollback().await;
                    }
                    Err(_) => {
                        // Ignore rollback failure in error case
                    }
                }
                Err(e)
            }
        }
    }
}
