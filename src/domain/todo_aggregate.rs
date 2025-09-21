use anyhow::Result as AnyhowResult;
use derive_getters::{Dissolve, Getters};

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
    async fn create_todo(tx: &mut sqlx::Transaction<'_, sqlx::Postgres>, todo: Todo) -> AnyhowResult<Todo>;
    async fn find_todo_by_id(tx: &mut sqlx::Transaction<'_, sqlx::Postgres>, id: uuid::Uuid) -> AnyhowResult<Option<Todo>>;
    async fn list_todos(tx: &mut sqlx::Transaction<'_, sqlx::Postgres>) -> AnyhowResult<Vec<Todo>>;
    async fn update_todo(tx: &mut sqlx::Transaction<'_, sqlx::Postgres>, todo: Todo) -> AnyhowResult<Todo>;
    async fn delete_todo(tx: &mut sqlx::Transaction<'_, sqlx::Postgres>, id: uuid::Uuid) -> AnyhowResult<()>;
}
