use std::future::Future;
use std::pin::Pin;
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
    ) -> Pin<Box<dyn Future<Output = Result<Todo, Self::Error>> + Send + '_>>
    where
        Self: Send;

    fn find_by_id(
        &self,
        db_context: &Arc<Mutex<Self::DbContext>>,
        id: Uuid,
    ) -> Pin<Box<dyn Future<Output = Result<Option<Todo>, Self::Error>> + Send + '_>>
    where
        Self: Send;

    fn find_all(
        &self,
        db_context: &Arc<Mutex<Self::DbContext>>,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Todo>, Self::Error>> + Send + '_>>
    where
        Self: Send;

    fn update(
        &self,
        db_context: &Arc<Mutex<Self::DbContext>>,
        todo: Todo,
    ) -> Pin<Box<dyn Future<Output = Result<Todo, Self::Error>> + Send + '_>>
    where
        Self: Send;

    fn delete(
        &self,
        db_context: &Arc<Mutex<Self::DbContext>>,
        id: Uuid,
    ) -> Pin<Box<dyn Future<Output = Result<(), Self::Error>> + Send + '_>>
    where
        Self: Send;
}
