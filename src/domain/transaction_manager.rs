use sqlx::Postgres;

pub trait TransactionManager {
    #[allow(dead_code)]
    async fn transaction<T, E, F>(&self, f: F) -> Result<T, E>
    where
        F: for<'a> FnOnce(
            &'a mut sqlx::Transaction<'_, Postgres>,
        ) -> std::pin::Pin<
            Box<dyn std::future::Future<Output = Result<T, E>> + Send + 'a>,
        >,
        E: From<sqlx::Error>,
        T: Send,
        E: Send;
}

// TODO: 2025-10-15 23:27
// Postgres が表出している。
// DB という Generic な型パラメータを持たせたい。
pub trait TransactionManager2 {
    #[allow(dead_code)]
    async fn transaction<T, E, F, DB>(&self, f: F) -> Result<T, E>
    where
        F: for<'a> FnOnce(
            &'a mut DB,
        ) -> std::pin::Pin<
            Box<dyn std::future::Future<Output = Result<T, E>> + Send + 'a>,
        >,
        E: From<sqlx::Error>,
        T: Send,
        E: Send;
}
