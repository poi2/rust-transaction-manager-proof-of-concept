use infrastructure::transaction_manager::db_context::DBContext;

mod domain;
mod infrastructure;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let conn_str =
        std::env::var("DATABASE_URL").expect("Env var DATABASE_URL is required for this example.");
    let pool = sqlx::PgPool::connect(&conn_str).await?;

    // DBContext生成
    let db_context = DBContext::new(pool.clone());

    let todo_id = uuid::Uuid::new_v4();

    Ok(())
}
