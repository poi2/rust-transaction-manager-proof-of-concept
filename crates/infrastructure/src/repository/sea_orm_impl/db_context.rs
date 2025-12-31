use sea_orm::DatabaseTransaction;
use domain::db_context::DbContext;

/// SeaORM DatabaseTransaction wrapper
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
    type Tx = DatabaseTransaction;
    type Error = anyhow::Error;

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