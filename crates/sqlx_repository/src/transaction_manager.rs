use std::future::Future;
use std::sync::Arc;
use tokio::sync::Mutex;

use sqlx::{PgPool, Postgres, Transaction};

use domain::db_context::DbContextMutexGuard;
use domain::transaction_manager::TransactionManagerMutexGuard;

use crate::db_context::SqlxDbContextMutexGuard;

/// DBContext implementation using PostgreSQL
pub struct DBContext {
    pool: PgPool,
}

impl DBContext {
    pub fn new(pool: PgPool) -> Self {
        DBContext { pool }
    }
}

impl TransactionManagerMutexGuard for DBContext {
    type DbContext = SqlxDbContextMutexGuard<'static>;
    type Error = anyhow::Error;

    async fn transaction<T, F, Fut>(&self, f: F) -> Result<T, Self::Error>
    where
        F: FnOnce(Arc<Mutex<Self::DbContext>>) -> Fut + Send,
        Fut: Future<Output = Result<T, Self::Error>> + Send,
        T: Send,
    {
        let tx = self.pool.begin().await?;

        // ライフタイム問題の回避: unsafeを使用してstatic化
        let tx: Transaction<'static, Postgres> = unsafe { std::mem::transmute(tx) };

        let db_context = Arc::new(Mutex::new(SqlxDbContextMutexGuard::new(tx)));
        let db_context_clone = db_context.clone();

        match f(db_context).await {
            Ok(result) => {
                let mut guard = db_context_clone.lock().await;
                guard.commit().await?;
                Ok(result)
            }
            Err(e) => {
                let mut guard = db_context_clone.lock().await;
                let _ = guard.rollback().await;
                Err(e)
            }
        }
    }
}
