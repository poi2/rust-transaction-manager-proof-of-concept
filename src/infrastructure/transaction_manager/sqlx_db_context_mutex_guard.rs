use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

use sqlx::{Column, Postgres, Row, Transaction};

use crate::domain::db_context_mutex_guard::{DbContextMutexGuard, QueryRow, QueryValue};

/// sqlx::Transaction のMutexGuard版ラッパー
pub struct SqlxDbContextMutexGuard<'a> {
    transaction: Option<Transaction<'a, Postgres>>,
}

impl<'a> SqlxDbContextMutexGuard<'a> {
    pub fn new(transaction: Transaction<'a, Postgres>) -> Self {
        Self { transaction: Some(transaction) }
    }
}

impl<'a> DbContextMutexGuard for SqlxDbContextMutexGuard<'a> {
    type Error = anyhow::Error;

    fn execute_query<'b>(
        &'b mut self,
        sql: &'b str,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<QueryRow>, Self::Error>> + Send + 'b>> {
        Box::pin(async move {
            let transaction = self.transaction.as_mut()
                .ok_or_else(|| anyhow::anyhow!("Transaction already consumed"))?;
            let rows = sqlx::query(sql).fetch_all(&mut **transaction).await?;
            
            let mut result = Vec::new();
            for row in rows {
                let mut data = HashMap::new();
                
                // PostgreSQLの基本的な型をサポート
                for (i, column) in row.columns().iter().enumerate() {
                    let column_name = column.name().to_string();
                    
                    // 型に基づいて値を取得
                    if let Ok(value) = row.try_get::<i32, _>(i) {
                        data.insert(column_name, QueryValue::Int(value));
                    } else if let Ok(value) = row.try_get::<i64, _>(i) {
                        data.insert(column_name, QueryValue::Int(value as i32));
                    } else if let Ok(value) = row.try_get::<String, _>(i) {
                        data.insert(column_name, QueryValue::String(value));
                    } else {
                        data.insert(column_name, QueryValue::Null);
                    }
                }
                
                result.push(QueryRow { data });
            }
            
            Ok(result)
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