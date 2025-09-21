use crate::domain::todo_aggregate::{Todo, TodoRepositoryTrait};
use anyhow::Result as AnyhowResult;
use sqlx::{Acquire, PgPool, Postgres, Transaction, query, query_as};

pub struct TodoRepositoryImpl;

impl TodoRepositoryImpl {
    pub async fn create_todo(
        db: impl Acquire<'_, Database = Postgres>,
        todo: Todo,
    ) -> AnyhowResult<Todo> {
        let mut conn = db.acquire().await?;

        query!(
            "INSERT INTO todo (id, description) VALUES ($1, $2)",
            todo.id,
            todo.description
        )
        .execute(&mut *conn)
        .await?;

        Ok(todo)
    }

    pub async fn find_todo_by_id(
        db: impl Acquire<'_, Database = Postgres>,
        id: uuid::Uuid,
    ) -> AnyhowResult<Option<Todo>> {
        let mut conn = db.acquire().await?;

        let row = query_as!(Todo, "SELECT id, description FROM todo WHERE id = $1", id)
            .fetch_optional(&mut *conn)
            .await?;

        Ok(row)
    }

    pub async fn update_todo(
        db: impl Acquire<'_, Database = Postgres>,
        todo: Todo,
    ) -> AnyhowResult<Todo> {
        let mut conn = db.acquire().await?;

        query!(
            "UPDATE todo SET description = $2 WHERE id = $1",
            todo.id,
            todo.description
        )
        .execute(&mut *conn)
        .await?;

        Ok(todo)
    }

    pub async fn list_todos(
        db: impl Acquire<'_, Database = Postgres>,
    ) -> AnyhowResult<Vec<Todo>> {
        let mut conn = db.acquire().await?;

        let rows = query_as!(Todo, "SELECT id, description FROM todo ORDER BY id")
            .fetch_all(&mut *conn)
            .await?;

        Ok(rows)
    }

    pub async fn delete_todo(
        db: impl Acquire<'_, Database = Postgres>,
        id: uuid::Uuid,
    ) -> AnyhowResult<()> {
        let mut conn = db.acquire().await?;

        query!("DELETE FROM todo WHERE id = $1", id)
            .execute(&mut *conn)
            .await?;

        Ok(())
    }
}

impl TodoRepositoryTrait for TodoRepositoryImpl {
    async fn create_todo<'a, A>(acquire: A, todo: Todo) -> AnyhowResult<Todo>
    where
        A: Acquire<'a, Database = Postgres> + Send,
    {
        Self::create_todo(acquire, todo).await
    }

    async fn find_todo_by_id<'a, A>(acquire: A, id: uuid::Uuid) -> AnyhowResult<Option<Todo>>
    where
        A: Acquire<'a, Database = Postgres> + Send,
    {
        Self::find_todo_by_id(acquire, id).await
    }

    async fn list_todos<'a, A>(acquire: A) -> AnyhowResult<Vec<Todo>>
    where
        A: Acquire<'a, Database = Postgres> + Send,
    {
        Self::list_todos(acquire).await
    }

    async fn update_todo<'a, A>(acquire: A, todo: Todo) -> AnyhowResult<Todo>
    where
        A: Acquire<'a, Database = Postgres> + Send,
    {
        Self::update_todo(acquire, todo).await
    }

    async fn delete_todo<'a, A>(acquire: A, id: uuid::Uuid) -> AnyhowResult<()>
    where
        A: Acquire<'a, Database = Postgres> + Send,
    {
        Self::delete_todo(acquire, id).await
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
    async fn test_create_todo_with_pool(pool: PgPool) {
        let todo = generate_todo("test todo");
        let result = TodoRepositoryImpl::create_todo(&pool, todo.clone()).await;
        assert!(result.is_ok());
        let created_todo = result.unwrap();
        assert_eq!(created_todo.id, todo.id);
        assert_eq!(created_todo.description, todo.description);
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
    async fn test_find_todo_by_id_with_pool(pool: PgPool) {
        let todo = generate_todo("test todo");
        TodoRepositoryImpl::create_todo(&pool, todo.clone())
            .await
            .unwrap();

        let result = TodoRepositoryImpl::find_todo_by_id(&pool, todo.id).await;
        assert!(result.is_ok());
        let found_todo = result.unwrap();
        assert!(found_todo.is_some());
        let found_todo = found_todo.unwrap();
        assert_eq!(found_todo.id, todo.id);
        assert_eq!(found_todo.description, todo.description);
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
    async fn test_list_todos_with_pool(pool: PgPool) {
        let todo1 = generate_todo("test todo 1");
        let todo2 = generate_todo("test todo 2");
        TodoRepositoryImpl::create_todo(&pool, todo1.clone())
            .await
            .unwrap();
        TodoRepositoryImpl::create_todo(&pool, todo2.clone())
            .await
            .unwrap();

        let result = TodoRepositoryImpl::list_todos(&pool).await;
        assert!(result.is_ok());
        let todos = result.unwrap();
        assert!(todos.len() >= 2);
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
    async fn test_update_todo_with_pool(pool: PgPool) {
        let todo = generate_todo("Original description");
        TodoRepositoryImpl::create_todo(&pool, todo.clone())
            .await
            .unwrap();

        let mut updated_todo = todo.clone();
        updated_todo.set_description("Updated description".to_string());
        let result = TodoRepositoryImpl::update_todo(&pool, updated_todo.clone()).await;
        assert!(result.is_ok());

        let found_todo = TodoRepositoryImpl::find_todo_by_id(&pool, todo.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(found_todo.description, "Updated description");
    }

    #[sqlx::test]
    async fn test_update_todo_with_transaction(pool: PgPool) {
        let mut tx = pool.begin().await.unwrap();
        let todo = generate_todo("Original description");
        TodoRepositoryImpl::create_todo(&mut tx, todo.clone())
            .await
            .unwrap();

        let mut updated_todo = todo.clone();
        updated_todo.set_description("Updated description".to_string());
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
    async fn test_delete_todo_with_pool(pool: PgPool) {
        let todo = generate_todo("test todo");
        TodoRepositoryImpl::create_todo(&pool, todo.clone())
            .await
            .unwrap();

        let result = TodoRepositoryImpl::delete_todo(&pool, todo.id).await;
        assert!(result.is_ok());

        let found_todo = TodoRepositoryImpl::find_todo_by_id(&pool, todo.id)
            .await
            .unwrap();
        assert!(found_todo.is_none());
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
