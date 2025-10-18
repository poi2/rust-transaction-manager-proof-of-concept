use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::domain::db_context_mutex_guard::DbContextMutexGuard;

/// Todo Repository trait for MutexGuard pattern
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
}

/// Todo aggregate
#[derive(Debug, Clone)]
pub struct Todo {
    pub id: Uuid,
    pub description: String,
}

impl Todo {
    pub fn new(id: Uuid, description: String) -> Self {
        Self { id, description }
    }
}