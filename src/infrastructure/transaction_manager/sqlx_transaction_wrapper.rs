use std::future::Future;
use std::pin::Pin;

use sqlx::{Postgres, Transaction};

use crate::domain::database_transaction::{DatabaseRow, DatabaseTransaction};

/// sqlx::Rowのwrapper
pub struct SqlxRowWrapper {
    row: sqlx::postgres::PgRow,
}

impl SqlxRowWrapper {
    pub fn new(row: sqlx::postgres::PgRow) -> Self {
        Self { row }
    }
}

impl DatabaseRow for SqlxRowWrapper {
    fn get_i32(&self, column: &str) -> Result<i32, Box<dyn std::error::Error + Send + Sync>> {
        use sqlx::Row;
        let value: i32 = self.row.try_get(column)?;
        Ok(value)
    }

    fn get_string(&self, column: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        use sqlx::Row;
        let value: String = self.row.try_get(column)?;
        Ok(value)
    }
}

/// sqlx::Transactionのwrapper
pub struct SqlxTransactionWrapper<'a> {
    transaction: Transaction<'a, Postgres>,
}

impl<'a> SqlxTransactionWrapper<'a> {
    #[allow(dead_code)]
    pub fn new(transaction: Transaction<'a, Postgres>) -> Self {
        Self { transaction }
    }
}

impl<'a> DatabaseTransaction for SqlxTransactionWrapper<'a> {
    type Row = SqlxRowWrapper;
    type Error = anyhow::Error;

    fn execute_query<'b>(
        &'b mut self,
        sql: &'b str,
    ) -> Pin<Box<dyn Future<Output = Result<Self::Row, Self::Error>> + Send + 'b>> {
        Box::pin(async move {
            let row = sqlx::query(sql).fetch_one(&mut *self.transaction).await?;
            Ok(SqlxRowWrapper::new(row))
        })
    }

    async fn commit(self) -> Result<(), Self::Error> {
        self.transaction
            .commit()
            .await
            .map_err(|e| anyhow::anyhow!(e))?;
        Ok(())
    }

    async fn rollback(self) -> Result<(), Self::Error> {
        self.transaction
            .rollback()
            .await
            .map_err(|e| anyhow::anyhow!(e))?;
        Ok(())
    }
}
