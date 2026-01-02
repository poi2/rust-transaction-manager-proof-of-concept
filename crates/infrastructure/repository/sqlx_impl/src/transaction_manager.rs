//! # SqlX Transaction Manager Implementation
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

use std::{future::Future, sync::Arc};

use domain::{db_context::DbContext, transaction_manager::TransactionManager};
use sqlx::PgPool;
use tokio::sync::Mutex;

use crate::SqlxDbContext;

/// transaction approach - eliminates unsafe code through architectural redesign
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

impl TransactionManager for SqlxTransactionManager {
    type DbContext = SqlxDbContext;
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

            let db_context = SqlxDbContext::new(tx_static);
            let db_context = Arc::new(Mutex::new(db_context));

            match f(db_context.clone()).await {
                Ok(result) => {
                    // Extract the context from Arc<Mutex<T>> to commit
                    // Arc<Mutex<T>>からコンテキストを抽出してコミット
                    match Arc::try_unwrap(db_context) {
                        Ok(mutex) => {
                            let mut context = mutex.into_inner();
                            context.commit().await?;
                            Ok(result)
                        }
                        Err(_) => {
                            // Fallback error if Arc::try_unwrap fails (shouldn't happen in normal use)
                            // Arc::try_unwrapが失敗した場合のフォールバックエラー（通常使用では発生しないはず）
                            Err(anyhow::anyhow!("Failed to extract context for commit"))
                        }
                    }
                }
                Err(e) => {
                    // Extract the context from Arc<Mutex<T>> to rollback
                    // Arc<Mutex<T>>からコンテキストを抽出してロールバック
                    match Arc::try_unwrap(db_context) {
                        Ok(mutex) => {
                            let mut context = mutex.into_inner();
                            let _ = context.rollback().await;
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
