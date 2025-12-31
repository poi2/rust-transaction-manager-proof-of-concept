use std::sync::Arc;
use tokio::sync::Mutex;
use sqlx::PgPool;

use domain::transaction_manager::TransactionManager;
use crate::repository::sqlx_impl::db_context::SqlxDbContext;

/// sqlx TransactionManager implementation using PostgreSQL
pub struct SqlxTransactionManager {
    pool: PgPool,
}

impl SqlxTransactionManager {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl TransactionManager for SqlxTransactionManager {
    type DbContext = SqlxDbContext<'static>;
    type Error = anyhow::Error;

    async fn transaction<T, F, Fut>(&self, f: F) -> Result<T, Self::Error>
    where
        F: FnOnce(Arc<Mutex<Self::DbContext>>) -> Fut + Send,
        Fut: std::future::Future<Output = Result<T, Self::Error>> + Send,
        T: Send,
    {
        // Note: This is a simplified implementation for demonstration.
        // In practice, we'd need to handle lifetimes more carefully.
        let tx = self.pool.begin().await?;
        let db_context = unsafe {
            // This is unsafe and not recommended for production use
            // A proper implementation would use a different approach for lifetime management
            std::mem::transmute::<SqlxDbContext<'_>, SqlxDbContext<'static>>(SqlxDbContext::new(tx))
        };
        let db_context = Arc::new(Mutex::new(db_context));

        match f(db_context.clone()).await {
            Ok(result) => {
                let mut guard = db_context.lock().await;
                guard.commit().await?;
                Ok(result)
            }
            Err(e) => {
                let mut guard = db_context.lock().await;
                let _ = guard.rollback().await;
                Err(e)
            }
        }
    }
}