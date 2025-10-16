use std::future::Future;
use std::pin::Pin;

/// Domain層のDbContext抽象化（MutexGuard版）
pub trait DbContextMutexGuard: Send + Sync {
    type Error: Send + Sync + 'static;

    /// SQL実行メソッド
    fn execute_query<'a>(
        &'a mut self,
        sql: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<QueryRow>, Self::Error>> + Send + 'a>>;

    /// トランザクションcommit (通常はTransactionManager側で呼ばれる)
    async fn commit(&mut self) -> Result<(), Self::Error>;

    /// トランザクションrollback (通常はTransactionManager側で呼ばれる)
    async fn rollback(&mut self) -> Result<(), Self::Error>;
}

/// クエリ結果の行抽象化
#[derive(Debug)]
pub struct QueryRow {
    pub data: std::collections::HashMap<String, QueryValue>,
}

/// クエリ結果の値抽象化
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
            Some(_) => Err(anyhow::anyhow!("Column {} is not an integer", column)),
            None => Err(anyhow::anyhow!("Column {} not found", column)),
        }
    }

    pub fn get_string(&self, column: &str) -> anyhow::Result<String> {
        match self.data.get(column) {
            Some(QueryValue::String(value)) => Ok(value.clone()),
            Some(_) => Err(anyhow::anyhow!("Column {} is not a string", column)),
            None => Err(anyhow::anyhow!("Column {} not found", column)),
        }
    }
}