use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::db_context::DbContextMutexGuard;
use crate::todo_aggregate::Todo;

/// Todo Repository trait for MutexGuard pattern
#[allow(async_fn_in_trait)]
pub trait TodoRepositoryMutexGuard {
    type DbContext: DbContextMutexGuard;
    type Error: Send + Sync + 'static;

    async fn create(
        &self,
        db_context: &Arc<Mutex<Self::DbContext>>,
        id: Uuid,
        description: &str,
    ) -> Result<Todo, Self::Error>;

    async fn find_by_id(
        &self,
        db_context: &Arc<Mutex<Self::DbContext>>,
        id: Uuid,
    ) -> Result<Option<Todo>, Self::Error>;

    async fn find_all(
        &self,
        db_context: &Arc<Mutex<Self::DbContext>>,
    ) -> Result<Vec<Todo>, Self::Error>;

    async fn update(
        &self,
        db_context: &Arc<Mutex<Self::DbContext>>,
        todo: Todo,
    ) -> Result<Todo, Self::Error>;

    async fn delete(
        &self,
        db_context: &Arc<Mutex<Self::DbContext>>,
        id: Uuid,
    ) -> Result<(), Self::Error>;
}
