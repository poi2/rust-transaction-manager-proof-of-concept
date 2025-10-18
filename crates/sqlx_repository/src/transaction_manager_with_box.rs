use std::future::Future;
use std::sync::Arc;
use tokio::sync::Mutex;

use sqlx::{Error as SqlxError, Executor, PgPool, Postgres, Row, Transaction};

use domain::db_context::DbContext;
use domain::transaction_manager::TransactionManager;

/// Transaction trait for type erasure through Box<dyn>
/// This allows us to avoid lifetime issues by owning the transaction
#[async_trait::async_trait]
pub trait TransactionTrait: Send + Sync {
    /// Execute a raw SQL query and return database rows
    async fn execute_query(&mut self, sql: &str) -> Result<Vec<Box<dyn Row>>, SqlxError>;

    /// Commit the transaction (consumes self)
    async fn commit(self: Box<Self>) -> Result<(), SqlxError>;

    /// Rollback the transaction (consumes self)
    async fn rollback(self: Box<Self>) -> Result<(), SqlxError>;
}

/// Concrete implementation of TransactionTrait for sqlx PostgreSQL
struct SqlxTransactionWrapper {
    transaction: Option<Transaction<'static, Postgres>>,
}

impl SqlxTransactionWrapper {
    fn new(transaction: Transaction<'static, Postgres>) -> Self {
        Self {
            transaction: Some(transaction),
        }
    }
}

#[async_trait::async_trait]
impl TransactionTrait for SqlxTransactionWrapper {
    async fn execute_query(&mut self, sql: &str) -> Result<Vec<Box<dyn Row>>, SqlxError> {
        let tx = self
            .transaction
            .as_mut()
            .ok_or_else(|| SqlxError::Protocol("Transaction already consumed".to_string()))?;

        // ❌ 問題: Row trait は object-safe ではない
        // sqlx::Row には以下のメソッドがある:
        // - fn try_get<'r, T, I>(&'r self, index: I) -> Result<T, Error>
        // これは generic method で、trait object にできない

        let rows = sqlx::query(sql).fetch_all(&mut **tx).await?;

        // ❌ コンパイルエラー: Box<dyn Row> は作成できない
        // error[E0038]: the trait `sqlx::Row` cannot be made into an object
        // Ok(rows.into_iter().map(|row| Box::new(row) as Box<dyn Row>).collect())

        todo!("Row trait is not object-safe - cannot create Box<dyn Row>")
    }

    async fn commit(mut self: Box<Self>) -> Result<(), SqlxError> {
        if let Some(tx) = self.transaction.take() {
            tx.commit().await
        } else {
            Err(SqlxError::Protocol(
                "Transaction already consumed".to_string(),
            ))
        }
    }

    async fn rollback(mut self: Box<Self>) -> Result<(), SqlxError> {
        if let Some(tx) = self.transaction.take() {
            tx.rollback().await
        } else {
            Err(SqlxError::Protocol(
                "Transaction already consumed".to_string(),
            ))
        }
    }
}

/// BoxedDbContext wrapping a boxed transaction trait
pub struct BoxedDbContext {
    transaction: Option<Box<dyn TransactionTrait>>,
}

impl BoxedDbContext {
    pub fn new(transaction: Box<dyn TransactionTrait>) -> Self {
        Self {
            transaction: Some(transaction),
        }
    }
}

impl DbContext for BoxedDbContext {
    // ❌ 問題: 型消去により、具体的なTransaction型を返せない
    // get_transaction() は &mut Self::Tx を返す必要があるが、
    // Box<dyn TransactionTrait> から &mut Transaction を取り出せない
    type Tx = Box<dyn TransactionTrait>;
    type Error = anyhow::Error;

    fn get_transaction(&mut self) -> &mut Self::Tx {
        // ❌ 設計上の問題:
        // Repository実装は sqlx::query!() マクロを使いたいが、
        // Box<dyn TransactionTrait> では sqlx の具体的な Transaction 型にアクセスできない
        self.transaction
            .as_mut()
            .expect("Transaction already consumed")
    }

    async fn commit(&mut self) -> Result<(), Self::Error> {
        if let Some(tx) = self.transaction.take() {
            tx.commit().await?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Transaction already consumed"))
        }
    }

    async fn rollback(&mut self) -> Result<(), Self::Error> {
        if let Some(tx) = self.transaction.take() {
            tx.rollback().await?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Transaction already consumed"))
        }
    }
}

/// TransactionManager implementation using Box<dyn> pattern
pub struct BoxedDBContext {
    pool: PgPool,
}

impl BoxedDBContext {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl TransactionManager for BoxedDBContext {
    type DbContext = BoxedDbContext;
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

            // ❌ 依然としてライフタイム問題: Transaction<'_, Postgres> を 'static にする必要
            // unsafe が必要な根本原因は変わらない
            let tx: Transaction<'static, Postgres> = unsafe { std::mem::transmute(tx) };

            let wrapper = SqlxTransactionWrapper::new(tx);
            let boxed_transaction: Box<dyn TransactionTrait> = Box::new(wrapper);
            let db_context = BoxedDbContext::new(boxed_transaction);
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

/*
=== 実装困難な理由の詳細分析 ===

1. **Row trait の object-safety 問題**
   - sqlx::Row は generic method を持つため trait object にできない
   - Box<dyn Row> が作成不可能
   - 代替案: 独自のRowWrapper を作成する必要があるが、型安全性が失われる

2. **get_transaction() の設計矛盾**
   - Repository は sqlx::query!() を使いたい
   - しかし Box<dyn TransactionTrait> では具体的な sqlx::Transaction にアクセス不可
   - 型消去により、元の型安全性が失われる

3. **ライフタイム問題の本質的な残存**
   - Box化してもTransaction<'_, Postgres> は依然として借用
   - 'static 化には依然として unsafe が必要
   - 根本解決にはならない

4. **複雑性の増加**
   - TransactionTrait の定義と実装
   - 型消去による抽象化レイヤー
   - デバッグ困難性の増加
   - パフォーマンスオーバーヘッド

=== 結論 ===
アプローチ2（Box化）は理論的には可能ですが、以下の理由で実用的ではありません：

1. Row trait の object-safety 制約により、完全な型消去が困難
2. get_transaction() で返される型が抽象化され、sqlx固有の機能が使えない
3. ライフタイム問題の根本解決にならない（依然として unsafe 必要）
4. 設計複雑性とパフォーマンス低下のトレードオフが見合わない

現在の unsafe transmute アプローチが最も実用的な解決策である理由が明確になります。
*/
