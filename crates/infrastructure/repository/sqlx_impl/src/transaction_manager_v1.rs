use sqlx::PgPool;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::db_context::SqlxDbContext;
use domain::{db_context::DbContext, transaction_manager::TransactionManager};

/// Original sqlx TransactionManager implementation (V1) - Has unsafe code issues
/// 元のsqlxトランザクションマネージャー実装 (V1) - unsafeコードの問題あり
///
/// ## ⚠️ WARNING / 警告
/// This implementation contains unsafe lifetime transmutation that can cause:
/// この実装には以下を引き起こす可能性があるunsafeライフタイム変換が含まれています:
///
/// - Dangling pointer issues / ダングリングポインタの問題
/// - Memory safety violations / メモリ安全性違反
/// - Unpredictable behavior across await points / awaitポイント間での予測不可能な動作
///
/// **DO NOT USE IN PRODUCTION** / **本番環境では使用しないでください**
pub struct SqlxTransactionManagerV1 {
    pool: PgPool,
}

impl SqlxTransactionManagerV1 {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl TransactionManager for SqlxTransactionManagerV1 {
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
        // This is unsafe and not recommended for production use
        // A proper implementation would use a different approach for lifetime management
        let db_context = unsafe {
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
