//! # SqlX Transaction Manager Implementation
//!
//! Transaction management implementation using ownership patterns with minimal unsafe code.
//!
//! ## Design Philosophy
//!
//! Instead of fighting with lifetimes through unsafe transmutation, this approach
//! redesigns the transaction management to work with ownership from the ground up.
//!
//! ## Key Architectural Changes
//!
//! ### 1. Ownership-based Transaction Handling
//! - Transactions are owned rather than borrowed
//! - Temporary Arc<Mutex<T>> for compatibility, reclaimed via try_unwrap
//! - Functional composition over imperative mutation
//!
//! ### 2. Consuming Operations
//! - commit() and rollback() consume self
//! - Prevents accidental reuse after transaction end
//! - Compile-time enforcement of transaction lifecycle
//!
//! ### 3. Reduced Unsafe Surface Area
//! - Unsafe code limited to initial setup
//! - Clear ownership boundaries
//! - Predictable memory management
//!
//! ---
//!
//! # 所有権ベースSqlXトランザクションマネージャー実装
//!
//! 最小限のunsafeコードで所有権パターンを使用したトランザクション管理の実装です。
//!
//! ## 設計哲学
//!
//! unsafeな変換でライフタイムと戦うのではなく、ゼロから所有権で動作するように
//! トランザクション管理を再設計します。
//!
//! ## 主要なアーキテクチャ変更
//!
//! ### 1. 所有権ベースのトランザクション処理
//! - トランザクションは借用ではなく所有
//! - 互換性のため一時的にArc<Mutex<T>>を使用、try_unwrapで所有権を取り戻す
//! - 命令的変更より関数合成
//!
//! ### 2. 消費型操作
//! - commit()とrollback()はselfを消費
//! - トランザクション終了後の誤った再利用を防止
//! - トランザクションライフサイクルのコンパイル時強制
//!
//! ### 3. Unsafeな範囲の削減
//! - Unsafeコードは初期設定に限定
//! - 明確な所有権境界
//! - 予測可能なメモリ管理

use std::{future::Future, sync::Arc};

use domain::{db_context::DbContext, transaction_manager::TransactionManager};
use sqlx::PgPool;
use tokio::sync::Mutex;

use crate::SqlxDbContext;

/// Ownership-based transaction manager implementation.
/// This implementation takes ownership of transactions instead of borrowing.
///
/// ## Key Design Points
///
/// 1. **Temporary shared state, reclaimed ownership** - Arc<Mutex<T>> used temporarily,
///    ownership reclaimed via Arc::try_unwrap
/// 2. **Minimal unsafe transmutation** - Only at initialization
/// 3. **Consuming operations** - commit/rollback take ownership after reclaiming from Arc
/// 4. **No long-lived shared mutable state** - Arc is unwrapped before transaction completion
///
/// ## Architecture
///
/// - DbContext owns the transaction rather than borrowing it
/// - Repository methods work with owned transactions (via Arc<Mutex> for compatibility)
/// - Transaction lifetime is explicitly managed through ownership
///
/// ---
///
/// オーナーシップベースのトランザクションマネージャー実装。
/// この実装は借用の代わりにトランザクションの所有権を取得します。
///
/// ## 主要な設計ポイント
///
/// 1. **一時的な共有状態、所有権の回収** - Arc<Mutex<T>>を一時的に使用、
///    Arc::try_unwrapで所有権を回収
/// 2. **最小限のunsafe変換** - 初期化時のみ
/// 3. **消費型操作** - commit/rollbackはArcから回収後に所有権を取得
/// 4. **長期的な共有可変状態なし** - トランザクション完了前にArcをアンラップ
///
/// ## アーキテクチャ
///
/// - DbContextは借用ではなくトランザクションを所有する
/// - リポジトリメソッドは所有されたトランザクションで動作（互換性のためArc<Mutex>経由）
/// - トランザクションライフタイムは所有権を通じて明示的に管理される
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
            //
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
                    //
                    // Arc<Mutex<T>>からコンテキストを抽出してコミット
                    match Arc::try_unwrap(db_context) {
                        Ok(mutex) => {
                            let context = mutex.into_inner();
                            context.commit().await?;
                            Ok(result)
                        }
                        Err(_) => {
                            // Fallback error if Arc::try_unwrap fails (shouldn't happen in normal use)
                            //
                            // Arc::try_unwrapが失敗した場合のフォールバックエラー（通常使用では発生しないはず）
                            Err(anyhow::anyhow!("Failed to extract context for commit"))
                        }
                    }
                }
                Err(e) => {
                    // Extract the context from Arc<Mutex<T>> to rollback
                    //
                    // Arc<Mutex<T>>からコンテキストを抽出してロールバック
                    match Arc::try_unwrap(db_context) {
                        Ok(mutex) => {
                            let context = mutex.into_inner();
                            let _ = context.rollback().await;
                        }
                        Err(_) => {
                            // Ignore rollback failure in error case
                            //
                            // エラーケースではロールバック失敗を無視
                        }
                    }
                    Err(e)
                }
            }
        }
    }
}
