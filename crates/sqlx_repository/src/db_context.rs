use sqlx::{Postgres, Transaction};

use domain::db_context::DbContext;

/// sqlx::Transaction wrapper for  pattern
pub struct SqlxDbContext<'a> {
    transaction: Option<Transaction<'a, Postgres>>,
}

impl<'a> SqlxDbContext<'a> {
    pub fn new(transaction: Transaction<'a, Postgres>) -> Self {
        Self {
            transaction: Some(transaction),
        }
    }
}

impl<'a> DbContext for SqlxDbContext<'a> {
    /// PostgreSQL transaction type from sqlx
    /// This provides direct access to sqlx::Transaction<Postgres> for:
    /// - Using query!() macros with compile-time SQL validation
    /// - Full PostgreSQL type safety and feature access
    /// - Native sqlx operations without abstraction overhead
    type Tx = Transaction<'a, Postgres>;
    type Error = anyhow::Error;

    /// Get mutable reference to the underlying sqlx transaction
    ///
    /// This allows repository implementations to use sqlx::query!() macros:
    /// ```rust
    /// let txn = guard.get_transaction();
    /// let result = sqlx::query!("SELECT id, name FROM users WHERE id = $1", user_id)
    ///     .fetch_optional(&mut **txn)
    ///     .await?;
    /// ```
    fn get_transaction(&mut self) -> &mut Self::Tx {
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
