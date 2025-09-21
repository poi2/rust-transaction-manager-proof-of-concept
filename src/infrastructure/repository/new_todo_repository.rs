use crate::domain::todo_aggregate::{NewTodoRepositoryTrait, PgTransactionManager, Todo};
use crate::infrastructure::transaction_manager::new_transaction_manager::NewTransactionManager;
use anyhow::Result as AnyhowResult;
use sqlx::{PgPool, Postgres, Transaction, query, query_as};
use std::sync::Arc;

pub struct NewTodoRepositoryImpl;

impl NewTodoRepositoryImpl {
    pub fn new() -> Self {
        Self {}
    }
}

// 型エイリアス使用でのRepository実装
#[async_trait::async_trait]
impl NewTodoRepositoryTrait for NewTodoRepositoryImpl {
    async fn create_todo(
        &self,
        txn_mgr: &PgTransactionManager<'_>,
        todo: Todo,
    ) -> AnyhowResult<Todo> {
        if txn_mgr.is_transaction_started().await {
            // Transaction内で実行
            let mut tx_guard = txn_mgr.get_transaction().await?;
            let tx = tx_guard.as_mut().unwrap();
            query!(
                "INSERT INTO todo (id, description) VALUES ($1, $2)",
                todo.id,
                todo.description
            )
            .execute(&mut **tx)
            .await?;
        } else {
            // 単発Connection実行
            let pool = txn_mgr.get_connection().await?;
            query!(
                "INSERT INTO todo (id, description) VALUES ($1, $2)",
                todo.id,
                todo.description
            )
            .execute(&*pool)
            .await?;
        }

        Ok(todo)
    }

    async fn find_todo_by_id(
        &self,
        txn_mgr: &PgTransactionManager<'_>,
        id: uuid::Uuid,
    ) -> AnyhowResult<Option<Todo>> {
        if txn_mgr.is_transaction_started().await {
            // Transaction内で実行
            let mut tx_guard = txn_mgr.get_transaction().await?;
            let tx = tx_guard.as_mut().unwrap();
            let row = query_as!(Todo, "SELECT id, description FROM todo WHERE id = $1", id)
                .fetch_optional(&mut **tx)
                .await?;
            Ok(row)
        } else {
            // 単発Connection実行
            let pool = txn_mgr.get_connection().await?;
            let row = query_as!(Todo, "SELECT id, description FROM todo WHERE id = $1", id)
                .fetch_optional(&*pool)
                .await?;
            Ok(row)
        }
    }

    async fn list_todos(&self, txn_mgr: &PgTransactionManager<'_>) -> AnyhowResult<Vec<Todo>> {
        if txn_mgr.is_transaction_started().await {
            // Transaction内で実行
            let mut tx_guard = txn_mgr.get_transaction().await?;
            let tx = tx_guard.as_mut().unwrap();
            let rows = query_as!(Todo, "SELECT id, description FROM todo ORDER BY id")
                .fetch_all(&mut **tx)
                .await?;
            Ok(rows)
        } else {
            // 単発Connection実行
            let pool = txn_mgr.get_connection().await?;
            let rows = query_as!(Todo, "SELECT id, description FROM todo ORDER BY id")
                .fetch_all(&*pool)
                .await?;
            Ok(rows)
        }
    }

    async fn update_todo(
        &self,
        txn_mgr: &PgTransactionManager<'_>,
        todo: Todo,
    ) -> AnyhowResult<Todo> {
        if txn_mgr.is_transaction_started().await {
            // Transaction内で実行
            let mut tx_guard = txn_mgr.get_transaction().await?;
            let tx = tx_guard.as_mut().unwrap();
            query!(
                "UPDATE todo SET description = $2 WHERE id = $1",
                todo.id,
                todo.description
            )
            .execute(&mut **tx)
            .await?;
        } else {
            // 単発Connection実行
            let pool = txn_mgr.get_connection().await?;
            query!(
                "UPDATE todo SET description = $2 WHERE id = $1",
                todo.id,
                todo.description
            )
            .execute(&*pool)
            .await?;
        }

        Ok(todo)
    }

    async fn delete_todo(
        &self,
        txn_mgr: &PgTransactionManager<'_>,
        id: uuid::Uuid,
    ) -> AnyhowResult<()> {
        if txn_mgr.is_transaction_started().await {
            // Transaction内で実行
            let mut tx_guard = txn_mgr.get_transaction().await?;
            let tx = tx_guard.as_mut().unwrap();
            query!("DELETE FROM todo WHERE id = $1", id)
                .execute(&mut **tx)
                .await?;
        } else {
            // 単発Connection実行
            let pool = txn_mgr.get_connection().await?;
            query!("DELETE FROM todo WHERE id = $1", id)
                .execute(&*pool)
                .await?;
        }

        Ok(())
    }
}
