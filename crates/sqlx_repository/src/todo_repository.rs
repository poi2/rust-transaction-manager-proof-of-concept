use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

use domain::db_context::DbContext;
use domain::todo_aggregate::Todo;
use domain::todo_repository::TodoRepository;

use crate::db_context::SqlxDbContext;

/// SqlxTodoRepository implementation using query() for type safety
#[derive(Clone)]
pub struct SqlxTodoRepository;

impl TodoRepository for SqlxTodoRepository {
    type DbContext = SqlxDbContext<'static>;
    type Error = anyhow::Error;

    async fn create(
        &self,
        db_context: &Arc<Mutex<Self::DbContext>>,
        id: Uuid,
        description: &str,
    ) -> Result<Todo, Self::Error> {
        let mut guard = db_context.lock().await;
        let txn = guard.get_transaction();

        sqlx::query("INSERT INTO todo (id, description) VALUES ($1, $2)")
            .bind(id)
            .bind(description)
            .execute(&mut **txn)
            .await?;

        Ok(Todo::new(id, description.to_string()))
    }

    async fn find_by_id(
        &self,
        db_context: &Arc<Mutex<Self::DbContext>>,
        id: Uuid,
    ) -> Result<Option<Todo>, Self::Error> {
        let mut guard = db_context.lock().await;
        let txn = guard.get_transaction();

        let result =
            sqlx::query_as::<_, (Uuid, String)>("SELECT id, description FROM todo WHERE id = $1")
                .bind(id)
                .fetch_optional(&mut **txn)
                .await?;

        Ok(result.map(|(id, description)| Todo::new(id, description)))
    }

    async fn find_all(
        &self,
        db_context: &Arc<Mutex<Self::DbContext>>,
    ) -> Result<Vec<Todo>, Self::Error> {
        let mut guard = db_context.lock().await;
        let txn = guard.get_transaction();

        let rows = sqlx::query_as::<_, (Uuid, String)>(
            "SELECT id, description FROM todo ORDER BY description",
        )
        .fetch_all(&mut **txn)
        .await?;

        Ok(rows
            .into_iter()
            .map(|(id, description)| Todo::new(id, description))
            .collect())
    }

    async fn update(
        &self,
        db_context: &Arc<Mutex<Self::DbContext>>,
        todo: Todo,
    ) -> Result<Todo, Self::Error> {
        let mut guard = db_context.lock().await;
        let txn = guard.get_transaction();

        sqlx::query("UPDATE todo SET description = $1 WHERE id = $2")
            .bind(todo.description())
            .bind(todo.id())
            .execute(&mut **txn)
            .await?;

        Ok(todo)
    }

    async fn delete(
        &self,
        db_context: &Arc<Mutex<Self::DbContext>>,
        id: Uuid,
    ) -> Result<(), Self::Error> {
        let mut guard = db_context.lock().await;
        let txn = guard.get_transaction();

        sqlx::query("DELETE FROM todo WHERE id = $1")
            .bind(id)
            .execute(&mut **txn)
            .await?;

        Ok(())
    }
}
