use futures::future::BoxFuture;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

use domain::db_context::DbContext;
use domain::todo_aggregate::Todo;
use domain::todo_repository::TodoRepository;

use crate::db_context::SqlxDbContext;

/// SqlxTodoRepository implementation using query!() macros for type safety
#[derive(Clone)]
pub struct SqlxTodoRepository;

impl TodoRepository for SqlxTodoRepository {
    type DbContext = SqlxDbContext<'static>;
    type Error = anyhow::Error;

    fn create(
        &self,
        db_context: &Arc<Mutex<Self::DbContext>>,
        id: Uuid,
        description: &str,
    ) -> BoxFuture<'_, Result<Todo, Self::Error>> {
        let db_context = db_context.clone();
        let description = description.to_string();
        Box::pin(async move {
            let mut guard = db_context.lock().await;
            let txn = guard.get_transaction();

            sqlx::query("INSERT INTO todo (id, description) VALUES ($1, $2)")
                .bind(id)
                .bind(&description)
                .execute(&mut **txn)
                .await?;

            Ok(Todo::new(id, description))
        })
    }

    fn find_by_id(
        &self,
        db_context: &Arc<Mutex<Self::DbContext>>,
        id: Uuid,
    ) -> BoxFuture<'_, Result<Option<Todo>, Self::Error>> {
        let db_context = db_context.clone();
        Box::pin(async move {
            let mut guard = db_context.lock().await;
            let txn = guard.get_transaction();

            let result = sqlx::query_as::<_, (Uuid, String)>(
                "SELECT id, description FROM todo WHERE id = $1",
            )
            .bind(id)
            .fetch_optional(&mut **txn)
            .await?;

            Ok(result.map(|(id, description)| Todo::new(id, description)))
        })
    }

    fn find_all(
        &self,
        db_context: &Arc<Mutex<Self::DbContext>>,
    ) -> BoxFuture<'_, Result<Vec<Todo>, Self::Error>> {
        let db_context = db_context.clone();
        Box::pin(async move {
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
        })
    }

    fn update(
        &self,
        db_context: &Arc<Mutex<Self::DbContext>>,
        todo: Todo,
    ) -> BoxFuture<'_, Result<Todo, Self::Error>> {
        let db_context = db_context.clone();
        Box::pin(async move {
            let mut guard = db_context.lock().await;
            let txn = guard.get_transaction();

            sqlx::query("UPDATE todo SET description = $1 WHERE id = $2")
                .bind(todo.description())
                .bind(todo.id())
                .execute(&mut **txn)
                .await?;

            Ok(todo)
        })
    }

    fn delete(
        &self,
        db_context: &Arc<Mutex<Self::DbContext>>,
        id: Uuid,
    ) -> BoxFuture<'_, Result<(), Self::Error>> {
        let db_context = db_context.clone();
        Box::pin(async move {
            let mut guard = db_context.lock().await;
            let txn = guard.get_transaction();

            sqlx::query("DELETE FROM todo WHERE id = $1")
                .bind(id)
                .execute(&mut **txn)
                .await?;

            Ok(())
        })
    }
}
