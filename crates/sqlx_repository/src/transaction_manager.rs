use std::future::Future;
use std::sync::Arc;
use tokio::sync::Mutex;

use sqlx::{PgPool, Postgres, Transaction};

use domain::db_context::DbContext;
use domain::transaction_manager::TransactionManager;

use crate::db_context::SqlxDbContext;

/// DBContext implementation using PostgreSQL
pub struct DBContext {
    pool: PgPool,
}

impl DBContext {
    pub fn new(pool: PgPool) -> Self {
        DBContext { pool }
    }
}

impl TransactionManager for DBContext {
    type DbContext = SqlxDbContext<'static>;
    type Error = anyhow::Error;

    async fn transaction<T, F, Fut>(&self, f: F) -> Result<T, Self::Error>
    where
        F: FnOnce(Arc<Mutex<Self::DbContext>>) -> Fut + Send,
        Fut: Future<Output = Result<T, Self::Error>> + Send,
        T: Send,
    {
        let tx = self.pool.begin().await?;

        // === SAFETY: ライフタイム transmute の技術的背景 ===
        //
        // **問題の本質:**
        // 1. トレイト制約: `type DbContext = SqlxDbContext<'static>` が要求される
        // 2. sqlx の現実: `pool.begin()` は `Transaction<'_, Postgres>` を返す
        // 3. ライフタイム不整合: 'static と実際のライフタイムが合わない
        //
        // **回避困難な理由:**
        // - GAT (Generic Associated Types) を使った理想形は現在のRustでは制約が多い:
        //   ```rust
        //   type DbContext<'a>: DbContext;  // 理想だが実装困難
        //   ```
        // - Owned Transaction パターンは型消去とパフォーマンス低下を伴う
        // - Connection Pool パターンは複数リポジトリでの一貫性保証が困難
        //
        // **安全性の根拠:**
        // 1. **スコープ制限**: transactionは`transaction()`関数内でのみ生存
        // 2. **RAII保証**: 関数終了時に必ずcommit/rollbackが呼ばれトランザクション消費
        // 3. **不変条件**: Transaction生存期間 ≤ 関数実行期間 (実際には厳密に制御されている)
        // 4. **メモリ安全**: トランザクションオブジェクトの実体は変更されない
        //
        // **将来の改善案:**
        // - Rust言語レベルでのGAT制約緩和
        // - sqlxでのowned transactionサポート
        // - 新しいasync trait設計パターンの確立
        let tx: Transaction<'static, Postgres> = unsafe { std::mem::transmute(tx) };

        let db_context = Arc::new(Mutex::new(SqlxDbContext::new(tx)));
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
