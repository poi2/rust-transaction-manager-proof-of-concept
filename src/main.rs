use anyhow::Result as AnyhowResult;
use domain::todo_aggregate::{Todo, TodoRepositoryTrait};
use domain::transaction_manager::TransactionManager;
use infrastructure::repository::todo_repository::TodoRepositoryImpl;
use infrastructure::transaction_manager::db_context::DBContext;
use sqlx::query;

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
    let original_description = "Learn Rust with SQLx".to_string();
    let updated_description = "Master Rust with SQLx".to_string();

    // 既存データをクリーンアップ
    let _ = query!(r#"DELETE FROM todo WHERE id = $1"#, todo_id)
        .execute(&pool)
        .await?;

    println!("Starting todo management example...");

    let new_todo = Todo::new(todo_id, original_description.clone());

    // トランザクション内でCRUD操作を実行
    let result: AnyhowResult<Todo> = db_context
        .transaction(|tx| {
            Box::pin(async move {
                // 1. Create Todo
                let created_todo = TodoRepositoryImpl::create_todo_tx(tx, new_todo).await?;
                println!("Created todo: {:?}", created_todo);

                // 2. Select Todo
                let found_todo = TodoRepositoryImpl::find_todo_by_id_tx(tx, todo_id).await?;
                println!("Found todo: {:?}", found_todo);

                // 3. Update Todo
                let updated_todo = Todo::new(todo_id, updated_description.clone());
                let updated_todo = TodoRepositoryImpl::update_todo_tx(tx, updated_todo).await?;
                println!("Updated todo: {:?}", updated_todo);

                // 4. Select updated Todo
                let updated_found_todo =
                    TodoRepositoryImpl::find_todo_by_id_tx(tx, todo_id).await?;
                println!("Updated found todo: {:?}", updated_found_todo);

                Ok(updated_todo)
            })
        })
        .await;

    match result {
        Ok(todo) => {
            println!("Transaction completed successfully!");
            println!("Final todo: {:?}", todo);

            // トランザクション外でも確認
            let final_check = TodoRepositoryImpl::find_todo_by_id(&pool, todo_id).await?;
            println!("Confirmed outside transaction: {:?}", final_check);
        }
        Err(e) => {
            println!("Transaction failed: {:?}", e);
        }
    }

    // クリーンアップ
    let _ = query!(r#"DELETE FROM todo WHERE id = $1"#, todo_id)
        .execute(&pool)
        .await?;

    println!("Example completed!");

    Ok(())
}
