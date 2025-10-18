use sqlx::{postgres::PgRow, Postgres, Transaction};

use domain::db_context::DbContextMutexGuard;

/// sqlx::Transaction wrapper for MutexGuard pattern
pub struct SqlxDbContextMutexGuard<'a> {
    transaction: Option<Transaction<'a, Postgres>>,
}

impl<'a> SqlxDbContextMutexGuard<'a> {
    pub fn new(transaction: Transaction<'a, Postgres>) -> Self {
        Self {
            transaction: Some(transaction),
        }
    }
}

impl<'a> DbContextMutexGuard for SqlxDbContextMutexGuard<'a> {
    /// PostgreSQL-specific row type from sqlx
    /// This provides access to all PostgreSQL data types including:
    /// - UUID, BIGINT, DECIMAL, BOOLEAN, TIMESTAMP, JSON, etc.
    /// - Custom types and arrays
    /// - Full type safety with compile-time SQL checking (when using sqlx::query!)
    type Row = PgRow;
    type Error = anyhow::Error;

    /// Execute raw SQL query against PostgreSQL transaction
    ///
    /// Returns native `sqlx::postgres::PgRow` instances, allowing:
    /// - `row.try_get::<Uuid, _>("id")?`
    /// - `row.try_get::<i64, _>("count")?`
    /// - `row.try_get::<serde_json::Value, _>("data")?`
    /// - Any PostgreSQL type supported by sqlx
    ///
    /// This implementation uses `Box::pin(async move { ... })` to satisfy
    /// the trait's Future requirements while maintaining type safety.
    fn execute_query<'b>(
        &'b mut self,
        sql: &'b str,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Vec<Self::Row>, Self::Error>> + Send + 'b>,
    > {
        Box::pin(async move {
            let transaction = self
                .transaction
                .as_mut()
                .ok_or_else(|| anyhow::anyhow!("Transaction already consumed"))?;
            let rows = sqlx::query(sql).fetch_all(&mut **transaction).await?;
            Ok(rows)
        })
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
