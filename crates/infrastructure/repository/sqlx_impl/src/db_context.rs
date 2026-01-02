use domain::db_context::DbContext;
use sqlx::{Postgres, Transaction};

pub struct SqlxDbContext {
    transaction: Transaction<'static, Postgres>,
}

impl SqlxDbContext {
    pub fn new(transaction: Transaction<'static, Postgres>) -> Self {
        Self { transaction }
    }
}

impl DbContext for SqlxDbContext {
    type Tx = Transaction<'static, Postgres>;
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
