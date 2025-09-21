use anyhow::Result as AnyhowResult;
use domain::todo_aggregate::Todo;
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

    println!("=== Transaction専用Repository実証 ===");

    // すべての操作をTransaction内で実行
    let crud_result: AnyhowResult<Todo> = db_context
        .transaction(|tx| {
            Box::pin(async move {
                // 1. Create Todo
                let new_todo = Todo::new(todo_id, original_description.clone());
                let created_todo = TodoRepositoryImpl::create_todo(tx, new_todo).await?;
                println!("1. Created todo: {:?}", created_todo);

                // 2. Select Todo
                let found_todo = TodoRepositoryImpl::find_todo_by_id(tx, todo_id).await?;
                println!("2. Found todo: {:?}", found_todo);

                // 3. Update Todo
                let updated_todo = Todo::new(todo_id, updated_description.clone());
                let updated_todo = TodoRepositoryImpl::update_todo(tx, updated_todo).await?;
                println!("3. Updated todo: {:?}", updated_todo);

                // 4. Select updated Todo
                let final_todo = TodoRepositoryImpl::find_todo_by_id(tx, todo_id).await?;
                println!("4. Final todo: {:?}", final_todo);

                Ok(updated_todo)
            })
        })
        .await;

    match crud_result {
        Ok(todo) => {
            println!("\n✅ Transaction内でのCRUD操作が全て成功！");
            println!("最終結果: {:?}", todo);
        }
        Err(e) => {
            println!("❌ Transaction失敗: {:?}", e);
            return Err(e.into());
        }
    }

    println!("\n=== Transaction専用Repository完成 ===");
    println!("✅ Repository は Transaction のみサポート");
    println!("✅ 複数操作も Transaction 内で正常動作");
    println!("✅ ライフタイム問題なし");

    // クリーンアップ
    let _ = query!(r#"DELETE FROM todo WHERE id = $1"#, todo_id)
        .execute(&pool)
        .await?;

    println!("Example completed!");

    Ok(())
}
