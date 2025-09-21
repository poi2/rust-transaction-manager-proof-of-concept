use crate::domain::todo_aggregate::{Todo, TodoRepositoryTrait};
use anyhow::Result as AnyhowResult;
use sqlx::{Postgres, Transaction, query, query_as};

pub struct TodoRepositoryImpl;

impl TodoRepositoryImpl {
    pub fn new() -> Self {
        Self {}
    }
}

impl TodoRepositoryTrait for TodoRepositoryImpl {
    async fn create_todo(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        todo: Todo,
    ) -> AnyhowResult<Todo> {
        query!(
            "INSERT INTO todo (id, description) VALUES ($1, $2)",
            todo.id,
            todo.description
        )
        .execute(&mut **tx)
        .await?;

        Ok(todo)
    }

    async fn find_todo_by_id(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        id: uuid::Uuid,
    ) -> AnyhowResult<Option<Todo>> {
        let row = query_as!(Todo, "SELECT id, description FROM todo WHERE id = $1", id)
            .fetch_optional(&mut **tx)
            .await?;

        Ok(row)
    }

    async fn list_todos(&self, tx: &mut Transaction<'_, Postgres>) -> AnyhowResult<Vec<Todo>> {
        let rows = query_as!(Todo, "SELECT id, description FROM todo ORDER BY id")
            .fetch_all(&mut **tx)
            .await?;

        Ok(rows)
    }

    async fn update_todo(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        todo: Todo,
    ) -> AnyhowResult<Todo> {
        query!(
            "UPDATE todo SET description = $2 WHERE id = $1",
            todo.id,
            todo.description
        )
        .execute(&mut **tx)
        .await?;

        Ok(todo)
    }

    async fn delete_todo(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        id: uuid::Uuid,
    ) -> AnyhowResult<()> {
        query!("DELETE FROM todo WHERE id = $1", id)
            .execute(&mut **tx)
            .await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::PgPool;

    fn generate_todo(description: &str) -> Todo {
        Todo::new(uuid::Uuid::new_v4(), description.to_string())
    }

    #[sqlx::test]
    async fn test_create_todo_with_transaction(pool: PgPool) {
        let mut tx = pool.begin().await.unwrap();
        let todo = generate_todo("test todo");
        let result = TodoRepositoryImpl::new()
            .create_todo(&mut tx, todo.clone())
            .await;
        assert!(result.is_ok());
        let created_todo = result.unwrap();
        assert_eq!(created_todo.id, todo.id);
        assert_eq!(created_todo.description, todo.description);
        tx.commit().await.unwrap();
    }

    #[sqlx::test]
    async fn test_find_todo_by_id_with_transaction(pool: PgPool) {
        let mut tx = pool.begin().await.unwrap();
        let todo = generate_todo("test todo");
        TodoRepositoryImpl::new()
            .create_todo(&mut tx, todo.clone())
            .await
            .unwrap();

        let result = TodoRepositoryImpl::new()
            .find_todo_by_id(&mut tx, todo.id)
            .await;
        assert!(result.is_ok());
        let found_todo = result.unwrap();
        assert!(found_todo.is_some());
        let found_todo = found_todo.unwrap();
        assert_eq!(found_todo.id, todo.id);
        assert_eq!(found_todo.description, todo.description);
        tx.commit().await.unwrap();
    }

    #[sqlx::test]
    async fn test_list_todos_with_transaction(pool: PgPool) {
        let mut tx = pool.begin().await.unwrap();
        let todo1 = generate_todo("test todo 1");
        let todo2 = generate_todo("test todo 2");
        TodoRepositoryImpl::new()
            .create_todo(&mut tx, todo1.clone())
            .await
            .unwrap();
        TodoRepositoryImpl::new()
            .create_todo(&mut tx, todo2.clone())
            .await
            .unwrap();

        let result = TodoRepositoryImpl::new().list_todos(&mut tx).await;
        assert!(result.is_ok());
        let todos = result.unwrap();
        assert!(todos.len() >= 2);
        tx.commit().await.unwrap();
    }

    #[sqlx::test]
    async fn test_update_todo_with_transaction(pool: PgPool) {
        let mut tx = pool.begin().await.unwrap();
        let todo = generate_todo("Original description");
        TodoRepositoryImpl::new()
            .create_todo(&mut tx, todo.clone())
            .await
            .unwrap();

        let updated_todo = Todo::new(todo.id, "Updated description".to_string());
        let result = TodoRepositoryImpl::new()
            .update_todo(&mut tx, updated_todo)
            .await;
        assert!(result.is_ok());

        let found_todo = TodoRepositoryImpl::new()
            .find_todo_by_id(&mut tx, todo.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(found_todo.description, "Updated description");
        tx.commit().await.unwrap();
    }

    #[sqlx::test]
    async fn test_delete_todo_with_transaction(pool: PgPool) {
        let mut tx = pool.begin().await.unwrap();
        let todo = generate_todo("test todo");
        TodoRepositoryImpl::new()
            .create_todo(&mut tx, todo.clone())
            .await
            .unwrap();

        let result = TodoRepositoryImpl::new()
            .delete_todo(&mut tx, todo.id)
            .await;
        assert!(result.is_ok());

        let found_todo = TodoRepositoryImpl::new()
            .find_todo_by_id(&mut tx, todo.id)
            .await
            .unwrap();
        assert!(found_todo.is_none());
        tx.commit().await.unwrap();
    }
}
