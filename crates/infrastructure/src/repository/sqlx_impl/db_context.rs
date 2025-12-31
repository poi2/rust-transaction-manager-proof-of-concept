use sqlx::{Postgres, Transaction};
use domain::db_context::DbContext;

/// sqlx::Transaction wrapper for PostgreSQL
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
    type Tx = Transaction<'a, Postgres>;
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