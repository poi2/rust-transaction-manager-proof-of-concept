use std::future::Future;
use std::pin::Pin;

/// Domain層のRow抽象化
pub trait DatabaseRow: Send + Sync {
    fn get_i32(&self, column: &str) -> Result<i32, Box<dyn std::error::Error + Send + Sync>>;
    fn get_string(&self, column: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>>;
}

/// Domain層のTransaction抽象化
pub trait DatabaseTransaction: Send {
    type Row: DatabaseRow;
    type Error: Send + Sync + 'static;

    fn execute_query<'a>(
        &'a mut self,
        sql: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<Self::Row, Self::Error>> + Send + 'a>>;

    async fn commit(self) -> Result<(), Self::Error>;

    async fn rollback(self) -> Result<(), Self::Error>;
}
