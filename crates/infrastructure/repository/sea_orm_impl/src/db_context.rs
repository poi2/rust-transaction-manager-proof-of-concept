use domain::db_context::DbContext;
use sea_orm::DatabaseTransaction;

/// SeaORM DatabaseTransaction wrapper
pub struct SeaOrmDbContext {
    transaction: DatabaseTransaction,
}

impl SeaOrmDbContext {
    pub fn new(transaction: DatabaseTransaction) -> Self {
        Self { transaction }
    }
}

impl DbContext for SeaOrmDbContext {
    type Tx = DatabaseTransaction;
    type Error = anyhow::Error;

    fn get_transaction(&mut self) -> &mut Self::Tx {
        &mut self.transaction
    }

    async fn commit(self) -> Result<(), Self::Error> {
        self.transaction.commit().await?;
        Ok(())
    }

    async fn rollback(self) -> Result<(), Self::Error> {
        self.transaction.rollback().await?;
        Ok(())
    }
}
