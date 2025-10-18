use std::future::Future;
use std::pin::Pin;

/// Type alias for complex query result future
pub type QueryResultFuture<'a, E> =
    Pin<Box<dyn Future<Output = Result<Vec<QueryRow>, E>> + Send + 'a>>;

/// Domain layer database context abstraction for MutexGuard pattern
#[allow(async_fn_in_trait)]
pub trait DbContextMutexGuard: Send + Sync {
    type Error: Send + Sync + 'static;

    /// Execute SQL query and return results
    fn execute_query<'a>(&'a mut self, sql: &'a str) -> QueryResultFuture<'a, Self::Error>;

    // Commit transaction (usually called by TransactionManager)
    async fn commit(&mut self) -> Result<(), Self::Error>;

    // Rollback transaction (usually called by TransactionManager)
    async fn rollback(&mut self) -> Result<(), Self::Error>;
}

/// Query result row abstraction
#[derive(Debug)]
pub struct QueryRow {
    pub data: std::collections::HashMap<String, QueryValue>,
}

/// Query result value abstraction
#[derive(Debug, Clone)]
pub enum QueryValue {
    Int(i32),
    String(String),
    Null,
}

impl QueryRow {
    pub fn get_i32(&self, column: &str) -> anyhow::Result<i32> {
        match self.data.get(column) {
            Some(QueryValue::Int(value)) => Ok(*value),
            Some(_) => Err(anyhow::anyhow!("Column {column} is not an integer")),
            None => Err(anyhow::anyhow!("Column {column} not found")),
        }
    }

    pub fn get_string(&self, column: &str) -> anyhow::Result<String> {
        match self.data.get(column) {
            Some(QueryValue::String(value)) => Ok(value.clone()),
            Some(_) => Err(anyhow::anyhow!("Column {column} is not a string")),
            None => Err(anyhow::anyhow!("Column {column} not found")),
        }
    }
}
