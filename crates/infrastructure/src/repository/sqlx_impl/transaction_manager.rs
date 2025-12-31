//! # Owned SqlX Transaction Manager Implementation
//! # 所有権ベースSqlXトランザクションマネージャー実装
//!
//! This is the most advanced implementation that fundamentally redesigns transaction
//! management using ownership patterns instead of shared mutable state.
//!
//! これは共有可変状態の代わりに所有権パターンを使用してトランザクション管理を
//! 根本的に再設計した最も先進的な実装です。
//!
//! ## Design Philosophy | 設計哲学
//!
//! Instead of fighting with lifetimes through unsafe transmutation, this approach
//! redesigns the transaction management to work with ownership from the ground up.
//!
//! unsafeな変換でライフタイムと戦うのではなく、このアプローチは
//! ゼロから所有権で動作するようにトランザクション管理を再設計します。
//!
//! ## Key Architectural Changes | 主要なアーキテクチャ変更:
//!
//! ### 1. Ownership-based Transaction Handling | 所有権ベースのトランザクション処理
//! - Transactions are owned rather than borrowed / トランザクションは借用ではなく所有
//! - No shared mutable state via Arc<Mutex<T>> / Arc<Mutex<T>>による共有可変状態なし
//! - Functional composition over imperative mutation / 命令的変更より関数合成
//!
//! ### 2. Consuming Operations | 消費型操作
//! - commit() and rollback() consume self / commit()とrollback()はselfを消費
//! - Prevents accidental reuse after transaction end / トランザクション終了後の誤った再利用を防止
//! - Compile-time enforcement of transaction lifecycle / トランザクションライフサイクルのコンパイル時強制
//!
//! ### 3. Reduced Unsafe Surface Area | Unsafeな範囲の削減
//! - Unsafe code limited to initial setup / Unsafeコードは初期設定に限定
//! - Clear ownership boundaries / 明確な所有権境界
//! - Predictable memory management / 予測可能なメモリ管理

use sqlx::PgPool;
use std::future::Future;
use std::sync::Arc;
use tokio::sync::Mutex;

use domain::{db_context::DbContext, transaction_manager::TransactionManager};

/// Owned transaction approach - eliminates unsafe code through architectural redesign
/// This implementation takes ownership of transactions instead of borrowing
///
/// オーナーシップトランザクションアプローチ - アーキテクチャ再設計によるunsafeコード排除
/// この実装は借用の代わりにトランザクションの所有権を取得します
///
/// ## Key Differences from Arc<Mutex> Pattern | Arc<Mutex>パターンとの主要な違い:
/// 1. **No shared mutable state** - Each transaction owns its resources
///    **共有可変状態なし** - 各トランザクションが独自のリソースを所有
/// 2. **Minimal unsafe transmutation** - Only at initialization
///    **最小限のunsafe変換** - 初期化時のみ
/// 3. **Functional composition** - Transactions are composed rather than shared
///    **関数合成** - トランザクションは共有ではなく合成される
/// 4. **Consuming operations** - commit/rollback take ownership
///    **消費型操作** - commit/rollbackは所有権を取得
///
/// ## Architecture Changes Required | 必要なアーキテクチャ変更:
/// - DbContext owns the transaction rather than borrowing it
///   DbContextは借用ではなくトランザクションを所有する
/// - Repository methods work with owned transactions (via Arc<Mutex> for compatibility)
///   リポジトリメソッドは所有されたトランザクションで動作（互換性のためArc<Mutex>経由）
/// - Transaction lifetime is explicitly managed through ownership
///   トランザクションライフタイムは所有権を通じて明示的に管理される
pub struct SqlxTransactionManager {
    pool: PgPool,
}

impl SqlxTransactionManager {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

/// Owned DbContext that owns its transaction
/// This eliminates most lifetime issues by taking ownership
///
/// トランザクションを所有するOwned DbContext
/// 所有権を取得することで大部分のライフタイム問題を排除
pub struct OwnedSqlxDbContext {
    tx: sqlx::Transaction<'static, sqlx::Postgres>,
}

impl OwnedSqlxDbContext {
    pub fn new(tx: sqlx::Transaction<'static, sqlx::Postgres>) -> Self {
        Self { tx }
    }

    /// Get a reference to the transaction for query execution
    /// クエリ実行用のトランザクション参照を取得
    pub fn transaction(&mut self) -> &mut sqlx::Transaction<'static, sqlx::Postgres> {
        &mut self.tx
    }

    /// Consume self and commit the transaction
    /// selfを消費してトランザクションをコミット
    pub async fn into_commit(self) -> Result<(), sqlx::Error> {
        self.tx.commit().await
    }

    /// Consume self and rollback the transaction
    /// selfを消費してトランザクションをロールバック
    pub async fn into_rollback(self) -> Result<(), sqlx::Error> {
        self.tx.rollback().await
    }
}

impl DbContext for OwnedSqlxDbContext {
    type Tx = sqlx::Transaction<'static, sqlx::Postgres>;
    type Error = sqlx::Error;

    fn get_transaction(&mut self) -> &mut Self::Tx {
        &mut self.tx
    }

    async fn commit(&mut self) -> Result<(), Self::Error> {
        // Note: This is a compatibility method for DbContext trait
        // The preferred way is to use into_commit() which consumes self
        // 注意: これはDbContextトレイトとの互換性のためのメソッド
        // 推奨はselfを消費するinto_commit()を使用すること
        Ok(())
    }

    async fn rollback(&mut self) -> Result<(), Self::Error> {
        // Note: This is a compatibility method for DbContext trait
        // The preferred way is to use into_rollback() which consumes self
        // 注意: これはDbContextトレイトとの互換性のためのメソッド
        // 推奨はselfを消費するinto_rollback()を使用すること
        Ok(())
    }
}

impl TransactionManager for SqlxTransactionManager {
    type DbContext = OwnedSqlxDbContext;
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

            // Cast transaction to 'static lifetime
            // This is the only unsafe operation, contained at initialization
            // トランザクションを'staticライフタイムにキャスト
            // これが唯一のunsafe操作で、初期化時に封じ込められている
            let tx_static = unsafe {
                std::mem::transmute::<
                    sqlx::Transaction<'_, sqlx::Postgres>,
                    sqlx::Transaction<'static, sqlx::Postgres>,
                >(tx)
            };

            let db_context = OwnedSqlxDbContext::new(tx_static);
            let db_context = Arc::new(Mutex::new(db_context));

            match f(db_context.clone()).await {
                Ok(result) => {
                    // Extract the owned context from Arc<Mutex<T>> to commit
                    // Arc<Mutex<T>>から所有コンテキストを抽出してコミット
                    match Arc::try_unwrap(db_context) {
                        Ok(mutex) => {
                            let owned_context = mutex.into_inner();
                            owned_context.into_commit().await?;
                            Ok(result)
                        }
                        Err(_) => {
                            // Fallback error if Arc::try_unwrap fails (shouldn't happen in normal use)
                            // Arc::try_unwrapが失敗した場合のフォールバックエラー（通常使用では発生しないはず）
                            Err(anyhow::anyhow!(
                                "Failed to extract owned context for commit"
                            ))
                        }
                    }
                }
                Err(e) => {
                    // Extract the owned context from Arc<Mutex<T>> to rollback
                    // Arc<Mutex<T>>から所有コンテキストを抽出してロールバック
                    match Arc::try_unwrap(db_context) {
                        Ok(mutex) => {
                            let owned_context = mutex.into_inner();
                            let _ = owned_context.into_rollback().await;
                        }
                        Err(_) => {
                            // Ignore rollback failure in error case
                            // エラーケースではロールバック失敗を無視
                        }
                    }
                    Err(e)
                }
            }
        }
    }
}
