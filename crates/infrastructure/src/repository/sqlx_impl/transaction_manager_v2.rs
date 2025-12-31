use sqlx::PgPool;
use std::future::Future;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::repository::sqlx_impl::db_context::SqlxDbContext;
use domain::{db_context::DbContext, transaction_manager::TransactionManager};

/// Safe sqlx TransactionManager implementation (V2) with improved safety
/// Uses `async move` blocks and better scope control to minimize unsafe code risks
///
/// 安全性を向上させたsqlxトランザクションマネージャー実装 (V2)
/// `async move`ブロックとより良いスコープ制御を使用してunsafeコードのリスクを最小化
///
/// ## Safety Improvements over V1 | V1からの安全性向上:
/// 1. **Encapsulation**: Unsafe code contained within `async move` block
///    **カプセル化**: `async move`ブロック内にunsafeコードを封じ込め
/// 2. **Deterministic lifetime**: Move semantics ensure predictable cleanup
///    **決定論的ライフタイム**: moveセマンティクスによる予測可能なクリーンアップ
/// 3. **Reduced surface area**: Single Future scope minimizes error possibilities
///    **縮小された影響範囲**: 単一Futureスコープでエラーの可能性を最小化
///
/// ## Remaining Limitations | 残る制限事項:
/// - Still requires unsafe transmutation / 依然としてunsafe変換が必要
/// - Arc<Mutex> pattern constraints remain / Arc<Mutex>パターンの制約は残る
pub struct SqlxTransactionManagerV2 {
    pool: PgPool,
}

impl SqlxTransactionManagerV2 {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl TransactionManager for SqlxTransactionManagerV2 {
    type DbContext = SqlxDbContext<'static>;
    type Error = anyhow::Error;

    fn transaction<T, F, Fut>(&self, f: F) -> impl Future<Output = Result<T, Self::Error>> + Send
    where
        F: FnOnce(Arc<Mutex<Self::DbContext>>) -> Fut + Send,
        Fut: Future<Output = Result<T, Self::Error>> + Send,
        T: Send,
    {
        let pool = self.pool.clone();

        async move {
            let tx = pool.begin().await?;

            // Still requires unsafe transmutation, but contained within this implementation
            let db_context = unsafe {
                std::mem::transmute::<SqlxDbContext<'_>, SqlxDbContext<'static>>(
                    SqlxDbContext::new(tx),
                )
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
}
