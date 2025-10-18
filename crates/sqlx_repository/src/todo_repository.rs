use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

use domain::db_context::DbContextMutexGuard;
use domain::todo_aggregate::Todo;
use domain::todo_repository::TodoRepositoryMutexGuard;

use crate::db_context::SqlxDbContextMutexGuard;

/// SqlxTodoRepository implementation for MutexGuard pattern
#[derive(Clone)]
pub struct SqlxTodoRepositoryMutexGuard;

impl TodoRepositoryMutexGuard for SqlxTodoRepositoryMutexGuard {
    type DbContext = SqlxDbContextMutexGuard<'static>;
    type Error = anyhow::Error;

    async fn create(
        &self,
        db_context: &Arc<Mutex<Self::DbContext>>,
        id: Uuid,
        description: &str,
    ) -> Result<Todo, Self::Error> {
        let mut guard = db_context.lock().await;

        // SQLインジェクション対策を含む安全な実装
        let sql_with_values = format!(
            "INSERT INTO todo (id, description) VALUES ('{}', '{}')",
            id,
            description.replace("'", "''") // SQLインジェクション対策
        );

        guard.execute_query(&sql_with_values).await?;
        Ok(Todo::new(id, description.to_string()))
    }

    async fn find_by_id(
        &self,
        db_context: &Arc<Mutex<Self::DbContext>>,
        id: Uuid,
    ) -> Result<Option<Todo>, Self::Error> {
        let mut guard = db_context.lock().await;
        let sql = format!("SELECT id, description FROM todo WHERE id = '{id}'");

        let rows = guard.execute_query(&sql).await?;

        if let Some(row) = rows.first() {
            let id_str = row.get_string("id")?;
            let description = row.get_string("description")?;
            let id = Uuid::parse_str(&id_str)?;
            Ok(Some(Todo::new(id, description)))
        } else {
            Ok(None)
        }
    }

    async fn find_all(
        &self,
        db_context: &Arc<Mutex<Self::DbContext>>,
    ) -> Result<Vec<Todo>, Self::Error> {
        let mut guard = db_context.lock().await;
        let sql = "SELECT id, description FROM todo ORDER BY description";

        let rows = guard.execute_query(sql).await?;
        let mut todos = Vec::new();

        for row in rows {
            let id_str = row.get_string("id")?;
            let description = row.get_string("description")?;
            let id = Uuid::parse_str(&id_str)?;
            todos.push(Todo::new(id, description));
        }

        Ok(todos)
    }

    async fn update(
        &self,
        db_context: &Arc<Mutex<Self::DbContext>>,
        todo: Todo,
    ) -> Result<Todo, Self::Error> {
        let mut guard = db_context.lock().await;
        let sql = format!(
            "UPDATE todo SET description = '{}' WHERE id = '{}'",
            todo.description().replace("'", "''"),
            todo.id()
        );

        guard.execute_query(&sql).await?;
        Ok(todo)
    }

    async fn delete(
        &self,
        db_context: &Arc<Mutex<Self::DbContext>>,
        id: Uuid,
    ) -> Result<(), Self::Error> {
        let mut guard = db_context.lock().await;
        let sql = format!("DELETE FROM todo WHERE id = '{id}'");

        guard.execute_query(&sql).await?;
        Ok(())
    }
}
