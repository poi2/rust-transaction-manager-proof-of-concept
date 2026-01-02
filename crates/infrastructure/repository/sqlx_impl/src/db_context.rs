use domain::db_context::DbContext;
use sqlx::{Postgres, Transaction};

pub struct SqlxDbContext {
    transaction: Option<Transaction<'static, Postgres>>,
}

impl SqlxDbContext {
    pub fn new(transaction: Transaction<'static, Postgres>) -> Self {
        Self {
            transaction: Some(transaction),
        }
    }
}

impl DbContext for SqlxDbContext {
    type Tx = Transaction<'static, Postgres>;
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
