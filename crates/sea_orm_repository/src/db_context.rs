use sea_orm::DatabaseTransaction;

use domain::db_context::DbContext;

/// SeaORM DatabaseTransaction wrapper for  pattern
pub struct SeaOrmDbContext {
    transaction: Option<DatabaseTransaction>,
}

impl SeaOrmDbContext {
    pub fn new(transaction: DatabaseTransaction) -> Self {
        Self {
            transaction: Some(transaction),
        }
    }
}

impl DbContext for SeaOrmDbContext {
    /// SeaORM DatabaseTransaction type
    /// This provides direct access to sea_orm::DatabaseTransaction for:
    /// - Using SeaORM's type-safe entity operations
    /// - Full SeaORM feature access (relations, active models, etc.)
    /// - Native SeaORM operations without abstraction overhead
    type Tx = DatabaseTransaction;
    type Error = anyhow::Error;

    /// Get mutable reference to the underlying SeaORM transaction
    ///
    /// This allows repository implementations to use SeaORM entity operations:
    /// ```rust,ignore
    /// let txn = guard.get_transaction();
    /// let result = TodoEntity::find()
    ///     .filter(todo::Column::Id.eq(id))
    ///     .one(txn)
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
