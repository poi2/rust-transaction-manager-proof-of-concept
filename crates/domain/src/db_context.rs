use std::future::Future;
use std::pin::Pin;

/// Domain layer database context abstraction for MutexGuard pattern
///
/// This trait provides database-agnostic transaction management while preserving
/// the ability to return database-specific row types for maximum flexibility.
#[allow(async_fn_in_trait)]
pub trait DbContextMutexGuard: Send + Sync {
    /// Database-specific row type (e.g., sqlx::postgres::PgRow)
    /// This allows users to access all native database types and features
    type Row;
    type Error: Send + Sync + 'static;

    /// Execute SQL query and return raw database rows
    ///
    /// # Technical Note: Why not async fn?
    ///
    /// This method cannot use `async fn` due to Rust's current type system limitations.
    /// When using `async fn` with associated types in traits:
    ///
    /// ```rust,ignore
    /// // This would cause compilation errors:
    /// async fn execute_query(&mut self, sql: &str) -> Result<Vec<Self::Row>, Self::Error>;
    /// ```
    ///
    /// The issue occurs because:
    /// 1. `async fn` desugars to `impl Future<Output = Result<Vec<Self::Row>, Self::Error>>`
    /// 2. The associated type `Self::Row` gets "captured" inside the Future
    /// 3. Rust's type system cannot properly handle the lifetime and type constraints
    ///
    /// Using explicit `Pin<Box<dyn Future<...>>>` allows us to:
    /// - Return database-specific row types (Self::Row)
    /// - Maintain type safety and lifetime correctness
    /// - Provide maximum flexibility for SQL operations
    ///
    /// Usage remains simple: `guard.execute_query("SELECT ...").await?`
    fn execute_query<'a>(
        &'a mut self,
        sql: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Self::Row>, Self::Error>> + Send + 'a>>;

    /// Commit transaction (usually called by TransactionManager)
    async fn commit(&mut self) -> Result<(), Self::Error>;

    /// Rollback transaction (usually called by TransactionManager)
    async fn rollback(&mut self) -> Result<(), Self::Error>;
}
