use std::{future::Future, pin::Pin};

use sqlx::{Error as SqlxError, PgPool, Postgres, Transaction};

use crate::domain::transaction_manager::TransactionManager;

#[allow(dead_code)]
pub struct DBContext {
    pool: PgPool,
}

impl DBContext {
    #[allow(dead_code)]
    pub fn new(pool: PgPool) -> Self {
        DBContext { pool }
    }
}

impl TransactionManager for DBContext {
    async fn transaction<T, E, F>(&self, f: F) -> Result<T, E>
    where
        F: for<'a> FnOnce(
            &'a mut Transaction<'_, Postgres>,
        ) -> Pin<Box<dyn Future<Output = Result<T, E>> + Send + 'a>>,
        E: From<SqlxError>,
        T: Send,
        E: Send,
    {
        let mut tx = self.pool.begin().await?;

        match f(&mut tx).await {
            Ok(result) => {
                tx.commit().await?;
                Ok(result)
            }
            Err(e) => {
                let _ = tx.rollback().await;
                Err(e)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use anyhow::Result as AnyhowResult;
    use sqlx::{PgPool, Postgres, query};

    trait SelectRepositoryTrait {
        async fn select_one(tx: &mut Transaction<'_, Postgres>) -> AnyhowResult<i32>;
        async fn select_invalid(tx: &mut Transaction<'_, Postgres>) -> AnyhowResult<i32>;
    }

    struct SelectRepositoryImpl;

    impl SelectRepositoryTrait for SelectRepositoryImpl {
        async fn select_one(tx: &mut Transaction<'_, Postgres>) -> AnyhowResult<i32> {
            let row = query!("SELECT 1 as value FROM todo")
                .fetch_one(&mut **tx)
                .await?;
            Ok(row.value.unwrap_or(0))
        }

        async fn select_invalid(tx: &mut Transaction<'_, Postgres>) -> AnyhowResult<i32> {
            let _row = sqlx::query("SELECT 1 FROM non_existent_table")
                .fetch_one(&mut **tx)
                .await?;

            unreachable!();
        }
    }

    async fn insert_todo(pool: &PgPool) -> AnyhowResult<()> {
        query!(
            r#"INSERT INTO todo (id, description)
                VALUES ( $1, $2 )
                "#,
            uuid::Uuid::new_v4(),
            "test todo",
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    #[sqlx::test]
    async fn test_transaction_success_with_repository(pool: PgPool) {
        insert_todo(&pool).await.unwrap();

        let db_context = DBContext::new(pool);

        let result: AnyhowResult<i32> = db_context
            .transaction(|tx| Box::pin(async move { SelectRepositoryImpl::select_one(tx).await }))
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1);
    }

    #[sqlx::test]
    async fn test_transaction_error_with_repository(pool: PgPool) {
        let db_context = DBContext::new(pool);

        let result: AnyhowResult<i32> = db_context
            .transaction(|tx| {
                Box::pin(async move { SelectRepositoryImpl::select_invalid(tx).await })
            })
            .await;

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err().downcast_ref::<SqlxError>(),
            Some(SqlxError::Database(_))
        ));
    }

    #[sqlx::test]
    async fn test_transaction_multiple_operations(pool: PgPool) {
        insert_todo(&pool).await.unwrap();

        let db_context = DBContext::new(pool);

        let result: AnyhowResult<i32> = db_context
            .transaction(|tx| {
                Box::pin(async move {
                    let first = SelectRepositoryImpl::select_one(tx).await?;
                    let second = SelectRepositoryImpl::select_one(tx).await?;
                    Ok(first + second)
                })
            })
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 2);
    }
}
