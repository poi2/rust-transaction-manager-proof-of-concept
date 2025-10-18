/// Domain layer database context abstraction for MutexGuard pattern
///
/// This trait provides database-agnostic transaction management while allowing
/// repository implementations to access native database transaction types
/// for maximum flexibility and type safety with query!() macros.
#[allow(async_fn_in_trait)]
pub trait DbContextMutexGuard: Send + Sync {
    /// Database-specific transaction type (e.g., sqlx::Transaction<Postgres>)
    /// This allows repository implementations to use query!() macros
    /// while keeping the domain layer database-agnostic
    type Tx: Send;
    type Error: Send + Sync + 'static;

    /// Get mutable reference to the underlying database transaction
    fn get_transaction(&mut self) -> &mut Self::Tx;

    /// Commit transaction (usually called by TransactionManager)
    async fn commit(&mut self) -> Result<(), Self::Error>;

    /// Rollback transaction (usually called by TransactionManager)
    async fn rollback(&mut self) -> Result<(), Self::Error>;
}
