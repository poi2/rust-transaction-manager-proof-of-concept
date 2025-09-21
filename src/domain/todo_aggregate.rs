use anyhow::Result as AnyhowResult;
use derive_getters::{Dissolve, Getters};
use sqlx::{Acquire, Postgres, Transaction};

use crate::infrastructure::transaction_manager::new_transaction_manager::NewTransactionManager;

pub trait PsqlAcquire<'c>: Acquire<'c, Database = Postgres> + Send {}
impl<'c, T> PsqlAcquire<'c> for T where T: Acquire<'c, Database = Postgres> + Send {}

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
    async fn create_todo(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        todo: Todo,
    ) -> AnyhowResult<Todo>;

    async fn find_todo_by_id(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        id: uuid::Uuid,
    ) -> AnyhowResult<Option<Todo>>;

    async fn list_todos(&self, tx: &mut Transaction<'_, Postgres>) -> AnyhowResult<Vec<Todo>>;

    async fn update_todo(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        todo: Todo,
    ) -> AnyhowResult<Todo>;

    async fn delete_todo(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        id: uuid::Uuid,
    ) -> AnyhowResult<()>;

    async fn create_todo2(&self, executor: impl PsqlAcquire<'_>, todo: Todo) -> AnyhowResult<Todo>;
}

// 新しいTransaction Manager用のRepository trait
#[allow(dead_code)]
#[async_trait::async_trait]
pub trait NewTodoRepositoryTrait<T, C> {
    async fn create_todo(
        &self,
        txn_mgr: &dyn NewTransactionManager<T, C>,
        todo: Todo,
    ) -> AnyhowResult<Todo>;

    async fn find_todo_by_id(
        &self,
        txn_mgr: &dyn NewTransactionManager<T, C>,
        id: uuid::Uuid,
    ) -> AnyhowResult<Option<Todo>>;

    async fn list_todos(
        &self,
        txn_mgr: &dyn NewTransactionManager<T, C>,
    ) -> AnyhowResult<Vec<Todo>>;

    async fn update_todo(
        &self,
        txn_mgr: &dyn NewTransactionManager<T, C>,
        todo: Todo,
    ) -> AnyhowResult<Todo>;

    async fn delete_todo(
        &self,
        txn_mgr: &dyn NewTransactionManager<T, C>,
        id: uuid::Uuid,
    ) -> AnyhowResult<()>;
}
