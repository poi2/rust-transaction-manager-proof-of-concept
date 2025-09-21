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
