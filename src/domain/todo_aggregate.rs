use anyhow::Result as AnyhowResult;
use derive_getters::{Dissolve, Getters};
use sqlx::{Acquire, Postgres};

#[allow(dead_code)]
#[derive(Debug, Clone, Dissolve, Getters)]
#[cfg_attr(any(test), derive(getset::Setters))]
#[cfg_attr(any(test), set = "pub")]
pub struct Todo {
    pub id: uuid::Uuid,
    pub description: String,
}

impl Todo {
    #[allow(dead_code)]
    pub fn new(id: uuid::Uuid, description: String) -> Self {
        Self { id, description }
    }
}

#[allow(dead_code)]
pub trait TodoRepositoryTrait {
    async fn create_todo<'a, A>(acquire: A, todo: Todo) -> AnyhowResult<Todo>
    where
        A: Acquire<'a, Database = Postgres> + Send;

    async fn find_todo_by_id<'a, A>(acquire: A, id: uuid::Uuid) -> AnyhowResult<Option<Todo>>
    where
        A: Acquire<'a, Database = Postgres> + Send;

    async fn list_todos<'a, A>(acquire: A) -> AnyhowResult<Vec<Todo>>
    where
        A: Acquire<'a, Database = Postgres> + Send;

    async fn update_todo<'a, A>(acquire: A, todo: Todo) -> AnyhowResult<Todo>
    where
        A: Acquire<'a, Database = Postgres> + Send;

    async fn delete_todo<'a, A>(acquire: A, id: uuid::Uuid) -> AnyhowResult<()>
    where
        A: Acquire<'a, Database = Postgres> + Send;
}
