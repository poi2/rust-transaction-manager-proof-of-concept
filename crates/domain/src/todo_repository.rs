use std::future::Future;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::db_context::DbContext;
use crate::todo_aggregate::Todo;

/// Todo Repository trait for Repository pattern
///
/// # async fn vs impl Future + Send
///
/// このトレイトでは `async fn` ではなく `fn -> impl Future + Send` を使用しています。
///
/// ## 理由
/// - `async fn` in trait は Rust 1.75+ で安定化されましたが、Send bound が自動的に保証されません
/// - マルチスレッド環境（tokio など）では Future が Send である必要があります
/// - `impl Future + Send` を使うことで、Send bound を明示的に指定できます
///
/// ## 比較
/// ```rust
/// // async fn - Send が保証されない
/// async fn create(&self, ...) -> Result<Todo, Self::Error>;
///
/// // impl Future + Send - Send を明示的に指定
/// fn create(&self, ...) -> impl Future<Output = Result<Todo, Self::Error>> + Send;
/// ```
pub trait TodoRepository: Send + Sync {
    type DbContext: DbContext;
    type Error: Send + Sync + 'static;

    fn create(
        &self,
        db_context: &Arc<Mutex<Self::DbContext>>,
        id: Uuid,
        description: &str,
    ) -> impl Future<Output = Result<Todo, Self::Error>> + Send
    where
        Self: Send;

    fn find_by_id(
        &self,
        db_context: &Arc<Mutex<Self::DbContext>>,
        id: Uuid,
    ) -> impl Future<Output = Result<Option<Todo>, Self::Error>> + Send
    where
        Self: Send;

    fn find_all(
        &self,
        db_context: &Arc<Mutex<Self::DbContext>>,
    ) -> impl Future<Output = Result<Vec<Todo>, Self::Error>> + Send
    where
        Self: Send;

    fn update(
        &self,
        db_context: &Arc<Mutex<Self::DbContext>>,
        todo: Todo,
    ) -> impl Future<Output = Result<Todo, Self::Error>> + Send
    where
        Self: Send;

    fn delete(
        &self,
        db_context: &Arc<Mutex<Self::DbContext>>,
        id: Uuid,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send
    where
        Self: Send;
}
