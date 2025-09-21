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

    // Pool を使った直接のCRUD操作で実証
    println!("=== Pool を使った操作 ===");

    // 1. Create Todo
    let new_todo = Todo::new(todo_id, original_description.clone());
    let created_todo = TodoRepositoryImpl::create_todo(&pool, new_todo).await?;
    println!("Created todo: {:?}", created_todo);

    // 2. Select Todo
    let found_todo = TodoRepositoryImpl::find_todo_by_id(&pool, todo_id).await?;
    println!("Found todo: {:?}", found_todo);

    // 3. Update Todo
    let updated_todo = Todo::new(todo_id, updated_description.clone());
    let updated_todo = TodoRepositoryImpl::update_todo(&pool, updated_todo).await?;
    println!("Updated todo: {:?}", updated_todo);

    // 4. Select updated Todo
    let updated_found_todo = TodoRepositoryImpl::find_todo_by_id(&pool, todo_id).await?;
    println!("Updated found todo: {:?}", updated_found_todo);

    println!("\n=== Transaction を使った操作 ===");

    // Transaction内での複数操作（DBContextの機能を活用）
    let tx_result: AnyhowResult<Todo> = db_context
        .transaction(|tx| {
            Box::pin(async move {
                // 新しいTodoを作成（Transactionを一度だけ使用）
                let tx_todo = Todo::new(uuid::Uuid::new_v4(), "Transaction Todo".to_string());
                let created = TodoRepositoryImpl::create_todo(&mut *tx, tx_todo).await?;
                println!("Transaction内で作成: {:?}", created);
                Ok(created)
            })
        })
        .await;

    match tx_result {
        Ok(todo) => {
            println!("Transaction操作成功: {:?}", todo);

            // Transactionで作成されたTodoをPoolで確認
            let confirmed = TodoRepositoryImpl::find_todo_by_id(&pool, todo.id).await?;
            println!("Poolで確認: {:?}", confirmed);
        }
        Err(e) => {
            println!("Transaction失敗: {:?}", e);
        }
    }

    println!("\n=== Acquire トレイトによる統一実証完了 ===");
    println!("✅ Pool: 複数操作可能");
    println!("✅ Transaction: 単一操作可能（テストで複数操作も確認済み）");
    println!("✅ 同じメソッドで両方に対応！");

    println!("\n=== 最終確認 ===");
    println!("最終的なTodo: {:?}", updated_found_todo);

    // クリーンアップ
    let _ = query!(r#"DELETE FROM todo WHERE id = $1"#, todo_id)
        .execute(&pool)
        .await?;

    println!("Example completed!");

    Ok(())
}
