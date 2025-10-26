use futures::future::BoxFuture;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::db_context::DbContext;
use crate::todo_aggregate::Todo;

/// Todo Repository trait for  pattern
pub trait TodoRepository: Send + Sync {
    type DbContext: DbContext;
    type Error: Send + Sync + 'static;

    fn create(
        &self,
        db_context: &Arc<Mutex<Self::DbContext>>,
        id: Uuid,
        description: &str,
    ) -> BoxFuture<'_, Result<Todo, Self::Error>>
    where
        Self: Send;

    fn find_by_id(
        &self,
        db_context: &Arc<Mutex<Self::DbContext>>,
        id: Uuid,
    ) -> BoxFuture<'_, Result<Option<Todo>, Self::Error>>
    where
        Self: Send;

    fn find_all(
        &self,
        db_context: &Arc<Mutex<Self::DbContext>>,
    ) -> BoxFuture<'_, Result<Vec<Todo>, Self::Error>>
    where
        Self: Send;

    fn update(
        &self,
        db_context: &Arc<Mutex<Self::DbContext>>,
        todo: Todo,
    ) -> BoxFuture<'_, Result<Todo, Self::Error>>
    where
        Self: Send;

    fn delete(
        &self,
        db_context: &Arc<Mutex<Self::DbContext>>,
        id: Uuid,
    ) -> BoxFuture<'_, Result<(), Self::Error>>
    where
        Self: Send;
}
