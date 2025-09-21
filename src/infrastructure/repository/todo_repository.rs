use crate::domain::todo_aggregate::{Todo, TodoRepositoryTrait};
use anyhow::Result as AnyhowResult;
use sqlx::{Postgres, Transaction, query, query_as};

pub struct TodoRepositoryImpl;

impl TodoRepositoryImpl {
    pub async fn create_todo(
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

    pub async fn find_todo_by_id(
        tx: &mut Transaction<'_, Postgres>,
        id: uuid::Uuid,
    ) -> AnyhowResult<Option<Todo>> {
        let row = query_as!(Todo, "SELECT id, description FROM todo WHERE id = $1", id)
            .fetch_optional(&mut **tx)
            .await?;

        Ok(row)
    }

    pub async fn update_todo(
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

    pub async fn list_todos(
        tx: &mut Transaction<'_, Postgres>,
    ) -> AnyhowResult<Vec<Todo>> {
        let rows = query_as!(Todo, "SELECT id, description FROM todo ORDER BY id")
            .fetch_all(&mut **tx)
            .await?;

        Ok(rows)
    }

    pub async fn delete_todo(
        tx: &mut Transaction<'_, Postgres>,
        id: uuid::Uuid,
    ) -> AnyhowResult<()> {
        query!("DELETE FROM todo WHERE id = $1", id)
            .execute(&mut **tx)
            .await?;

        Ok(())
    }
}

impl TodoRepositoryTrait for TodoRepositoryImpl {
    async fn create_todo(tx: &mut Transaction<'_, Postgres>, todo: Todo) -> AnyhowResult<Todo> {
        Self::create_todo(tx, todo).await
    }

    async fn find_todo_by_id(tx: &mut Transaction<'_, Postgres>, id: uuid::Uuid) -> AnyhowResult<Option<Todo>> {
        Self::find_todo_by_id(tx, id).await
    }

    async fn list_todos(tx: &mut Transaction<'_, Postgres>) -> AnyhowResult<Vec<Todo>> {
        Self::list_todos(tx).await
    }

    async fn update_todo(tx: &mut Transaction<'_, Postgres>, todo: Todo) -> AnyhowResult<Todo> {
        Self::update_todo(tx, todo).await
    }

    async fn delete_todo(tx: &mut Transaction<'_, Postgres>, id: uuid::Uuid) -> AnyhowResult<()> {
        Self::delete_todo(tx, id).await
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
        let result = TodoRepositoryImpl::create_todo(&mut tx, todo.clone()).await;
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
        TodoRepositoryImpl::create_todo(&mut tx, todo.clone())
            .await
            .unwrap();

        let result = TodoRepositoryImpl::find_todo_by_id(&mut tx, todo.id).await;
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
        TodoRepositoryImpl::create_todo(&mut tx, todo1.clone())
            .await
            .unwrap();
        TodoRepositoryImpl::create_todo(&mut tx, todo2.clone())
            .await
            .unwrap();

        let result = TodoRepositoryImpl::list_todos(&mut tx).await;
        assert!(result.is_ok());
        let todos = result.unwrap();
        assert!(todos.len() >= 2);
        tx.commit().await.unwrap();
    }

    #[sqlx::test]
    async fn test_update_todo_with_transaction(pool: PgPool) {
        let mut tx = pool.begin().await.unwrap();
        let todo = generate_todo("Original description");
        TodoRepositoryImpl::create_todo(&mut tx, todo.clone())
            .await
            .unwrap();

        let updated_todo = Todo::new(todo.id, "Updated description".to_string());
        let result = TodoRepositoryImpl::update_todo(&mut tx, updated_todo).await;
        assert!(result.is_ok());

        let found_todo = TodoRepositoryImpl::find_todo_by_id(&mut tx, todo.id)
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
        TodoRepositoryImpl::create_todo(&mut tx, todo.clone())
            .await
            .unwrap();

        let result = TodoRepositoryImpl::delete_todo(&mut tx, todo.id).await;
        assert!(result.is_ok());

        let found_todo = TodoRepositoryImpl::find_todo_by_id(&mut tx, todo.id)
            .await
            .unwrap();
        assert!(found_todo.is_none());
        tx.commit().await.unwrap();
    }
}
