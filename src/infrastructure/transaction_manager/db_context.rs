use std::{future::Future, pin::Pin};

use sqlx::{Error as SqlxError, PgPool, Postgres, Transaction};

use crate::domain::database_transaction::DatabaseTransaction;
use crate::domain::db_context_mutex_guard::DbContextMutexGuard;
use crate::domain::transaction_manager::{TransactionManager, TransactionManager3};
use crate::domain::transaction_manager_mutex_guard::TransactionManagerMutexGuard;
use crate::infrastructure::transaction_manager::sqlx_db_context_mutex_guard::SqlxDbContextMutexGuard;
use crate::infrastructure::transaction_manager::sqlx_transaction_wrapper::SqlxTransactionWrapper;
use std::sync::Arc;
use tokio::sync::Mutex;

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

impl TransactionManager3 for DBContext {
    type Error = anyhow::Error;
    type Row = SqlxTransactionWrapper<'static>;

    async fn transaction<T, F>(&self, f: F) -> Result<T, Self::Error>
    where
        F: for<'a> FnOnce(
            &'a mut Self::Row,
        )
            -> Pin<Box<dyn Future<Output = Result<T, Self::Error>> + Send + 'a>>,
        T: Send,
    {
        let tx = self.pool.begin().await?;
        let mut wrapper = SqlxTransactionWrapper::new(tx);

        match f(&mut wrapper).await {
            Ok(result) => {
                wrapper.commit().await?;
                Ok(result)
            }
            Err(e) => {
                let _ = wrapper.rollback().await;
                Err(e)
            }
        }
    }
}

impl TransactionManagerMutexGuard for DBContext {
    type DbContext = SqlxDbContextMutexGuard<'static>;
    type Error = anyhow::Error;

    async fn transaction<T, F, Fut>(&self, f: F) -> Result<T, Self::Error>
    where
        F: FnOnce(Arc<Mutex<Self::DbContext>>) -> Fut + Send,
        Fut: Future<Output = Result<T, Self::Error>> + Send,
        T: Send,
    {
        let tx = self.pool.begin().await?;

        // ライフタイム問題の回避: unsafeを使用してstatic化
        let tx: Transaction<'static, Postgres> = unsafe { std::mem::transmute(tx) };

        let db_context = Arc::new(Mutex::new(SqlxDbContextMutexGuard::new(tx)));
        let db_context_clone = db_context.clone();

        match f(db_context).await {
            Ok(result) => {
                let mut guard = db_context_clone.lock().await;
                guard.commit().await?;
                Ok(result)
            }
            Err(e) => {
                let mut guard = db_context_clone.lock().await;
                let _ = guard.rollback().await;
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

        let result: AnyhowResult<i32> = TransactionManager::transaction(&db_context, |tx| {
            Box::pin(async move { SelectRepositoryImpl::select_one(tx).await })
        })
        .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1);
    }

    #[sqlx::test]
    async fn test_transaction_error_with_repository(pool: PgPool) {
        let db_context = DBContext::new(pool);

        let result: AnyhowResult<i32> = TransactionManager::transaction(&db_context, |tx| {
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

        let result: AnyhowResult<i32> = TransactionManager::transaction(&db_context, |tx| {
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

#[cfg(test)]
mod tests_for_generic_pattern {
    use super::*;

    use anyhow::Result as AnyhowResult;
    use sqlx::{Database, query};

    trait SelectRepositoryTraitGenericPattern<DB: Database> {
        async fn select_one(tx: &mut Transaction<'_, DB>) -> AnyhowResult<i32>;
    }

    struct SelectRepositoryImplGenericPattern;

    impl SelectRepositoryTraitGenericPattern<Postgres> for SelectRepositoryImplGenericPattern {
        async fn select_one(tx: &mut Transaction<'_, Postgres>) -> AnyhowResult<i32> {
            let row = sqlx::query!("SELECT 1 as value FROM todo")
                .fetch_one(&mut **tx)
                .await?;
            Ok(row.value.unwrap_or(0))
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
    async fn test_transaction_multiple_operations_for_generic_pattern(pool: PgPool) {
        insert_todo(&pool).await.unwrap();

        let db_context = DBContext::new(pool);

        let result: AnyhowResult<i32> = TransactionManager::transaction(&db_context, |tx| {
            Box::pin(async move {
                let first = SelectRepositoryImplGenericPattern::select_one(tx).await?;
                let second = SelectRepositoryImplGenericPattern::select_one(tx).await?;
                Ok(first + second)
            })
        })
        .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 2);
    }
}

// TransactionWrapperのテスト
#[cfg(test)]
mod transaction_wrapper_tests {
    use super::*;

    use crate::domain::database_transaction::{DatabaseRow, DatabaseTransaction};
    use crate::infrastructure::transaction_manager::sqlx_transaction_wrapper::SqlxTransactionWrapper;
    use anyhow::Result as AnyhowResult;
    use sqlx::query;

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
    async fn test_transaction_wrapper_execute_query(pool: PgPool) {
        insert_todo(&pool).await.unwrap();

        let tx = pool.begin().await.unwrap();
        let mut wrapper = SqlxTransactionWrapper::new(tx);

        let row = wrapper.execute_query("SELECT 1 as value FROM todo").await;

        assert!(row.is_ok());
        let row = row.unwrap();
        let value = row.get_i32("value");
        assert!(value.is_ok());
        assert_eq!(value.unwrap(), 1);
    }

    #[sqlx::test]
    async fn test_transaction_wrapper_commit(pool: PgPool) {
        let tx = pool.begin().await.unwrap();
        let wrapper = SqlxTransactionWrapper::new(tx);

        let result = wrapper.commit().await;
        assert!(result.is_ok());
    }

    #[sqlx::test]
    async fn test_transaction_wrapper_rollback(pool: PgPool) {
        let tx = pool.begin().await.unwrap();
        let wrapper = SqlxTransactionWrapper::new(tx);

        let result = wrapper.rollback().await;
        assert!(result.is_ok());
    }
}

// UseCase向けのテスト
#[cfg(test)]
mod usecase_pattern_tests {
    use super::*;
    use crate::domain::database_transaction::{DatabaseRow, DatabaseTransaction};
    use anyhow::Result as AnyhowResult;
    use sqlx::{PgPool, query};

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

    // UseCase風の関数
    async fn todo_usecase(db_context: &DBContext) -> AnyhowResult<i32> {
        TransactionManager3::transaction(db_context, |tx| {
            Box::pin(async move {
                // Repository層の操作をシミュレート
                let row = tx.execute_query("SELECT 1 as value FROM todo").await?;
                let value = row.get_i32("value")?;
                Ok(value)
            })
        })
        .await
    }

    #[sqlx::test]
    async fn test_usecase_pattern(pool: PgPool) {
        insert_todo(&pool).await.unwrap();

        let db_context = DBContext::new(pool);

        // UseCase内でtransaction_wrapper.transaction({...})パターン
        let result = todo_usecase(&db_context).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1);
    }

    #[sqlx::test]
    async fn test_usecase_pattern_with_error(pool: PgPool) {
        let db_context = DBContext::new(pool);

        let result: Result<i32, anyhow::Error> =
            TransactionManager3::transaction(&db_context, |tx| {
                Box::pin(async move {
                    // 存在しないテーブルでエラーを発生させる
                    let _row = tx.execute_query("SELECT 1 FROM non_existent_table").await?;
                    Ok::<i32, anyhow::Error>(1)
                })
            })
            .await;

        assert!(result.is_err());
    }
}

// MutexGuard版のテスト
#[cfg(test)]
mod mutex_guard_tests {
    use super::*;
    use crate::domain::transaction_manager_mutex_guard::TransactionManagerMutexGuard;
    use anyhow::Result as AnyhowResult;
    use sqlx::{PgPool, query};

    async fn insert_todo(pool: &PgPool) -> AnyhowResult<()> {
        query!(
            r#"INSERT INTO todo (id, description)
                VALUES ( $1, $2 )
                "#,
            uuid::Uuid::new_v4(),
            "test todo for mutex guard",
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    #[sqlx::test]
    async fn test_mutex_guard_basic_usage(pool: PgPool) {
        insert_todo(&pool).await.unwrap();

        let db_context = DBContext::new(pool);

        let result: AnyhowResult<i32> =
            TransactionManagerMutexGuard::transaction(&db_context, |db_context| async move {
                let mut guard = db_context.lock().await;
                let rows = guard
                    .execute_query("SELECT COUNT(*) as count FROM todo")
                    .await?;

                if let Some(row) = rows.first() {
                    let count = row.get_i32("count")?;
                    Ok(count)
                } else {
                    Ok(0)
                }
            })
            .await;

        if let Err(e) = &result {
            println!("Error: {:?}", e);
        }
        assert!(result.is_ok());
        assert!(result.unwrap() > 0);
    }

    #[sqlx::test]
    async fn test_mutex_guard_multiple_operations(pool: PgPool) {
        insert_todo(&pool).await.unwrap();

        let db_context = DBContext::new(pool);

        let result: AnyhowResult<Vec<i32>> =
            TransactionManagerMutexGuard::transaction(&db_context, |db_context| async move {
                let mut results = Vec::new();

                // 1回目の呼び出し
                {
                    let mut guard = db_context.lock().await;
                    let rows = guard.execute_query("SELECT 1 as value").await?;
                    if let Some(row) = rows.first() {
                        results.push(row.get_i32("value")?);
                    }
                }

                // 2回目の呼び出し
                {
                    let mut guard = db_context.lock().await;
                    let rows = guard.execute_query("SELECT 2 as value").await?;
                    if let Some(row) = rows.first() {
                        results.push(row.get_i32("value")?);
                    }
                }

                Ok(results)
            })
            .await;

        assert!(result.is_ok());
        let values = result.unwrap();
        assert_eq!(values, vec![1, 2]);
    }

    #[sqlx::test]
    async fn test_mutex_guard_error_rollback(pool: PgPool) {
        let db_context = DBContext::new(pool);

        let result: AnyhowResult<i32> =
            TransactionManagerMutexGuard::transaction(&db_context, |db_context| async move {
                let mut guard = db_context.lock().await;
                // 存在しないテーブルでエラーを発生させる
                let _rows = guard
                    .execute_query("SELECT * FROM non_existent_table")
                    .await?;
                Ok(1)
            })
            .await;

        assert!(result.is_err());
    }

    // Repository trait for MutexGuard pattern
    trait TodoRepositoryMutexGuard {
        async fn create(
            &self,
            db_context: &Arc<Mutex<SqlxDbContextMutexGuard<'_>>>,
            description: &str,
        ) -> AnyhowResult<String>;
    }

    // Repository implementation
    struct SqlxTodoRepository;

    impl TodoRepositoryMutexGuard for SqlxTodoRepository {
        async fn create(
            &self,
            db_context: &Arc<Mutex<SqlxDbContextMutexGuard<'_>>>,
            description: &str,
        ) -> AnyhowResult<String> {
            let mut guard = db_context.lock().await;
            let rows = guard
                .execute_query("SELECT COUNT(*) as count FROM todo")
                .await?;

            if let Some(row) = rows.first() {
                let count = row.get_i32("count")?;
                Ok(format!("{}: {}", description, count))
            } else {
                Ok(description.to_string())
            }
        }
    }

    // UseCase風のテスト - memo.mdの要求仕様を検証
    #[sqlx::test]
    async fn test_usecase_pattern_verification(pool: PgPool) {
        insert_todo(&pool).await.unwrap();

        let db_context = DBContext::new(pool);
        let todo_repository = SqlxTodoRepository;

        // memo.mdで要求されたパターン
        let result: AnyhowResult<Vec<String>> =
            TransactionManagerMutexGuard::transaction(&db_context, |db_context| async move {
                // 1回目の呼び出し
                let todo1 = todo_repository.create(&db_context, "First Todo").await?;
                // 2回目の呼び出し
                let todo2 = todo_repository.create(&db_context, "Second Todo").await?;

                Ok(vec![todo1, todo2])
            })
            .await;

        assert!(result.is_ok());
        let todos = result.unwrap();
        assert_eq!(todos.len(), 2);
        assert!(todos[0].contains("First Todo"));
        assert!(todos[1].contains("Second Todo"));
    }
}
