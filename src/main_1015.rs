use anyhow::Result as AnyhowResult;
use domain::todo_aggregate::{NewTodoRepositoryTrait, Todo, TodoRepositoryTrait};
use domain::transaction_manager::TransactionManager;
use infrastructure::repository::new_todo_repository::NewTodoRepositoryImpl;
use infrastructure::repository::todo_repository::TodoRepositoryImpl;
use infrastructure::transaction_manager::db_context::DBContext;
use infrastructure::transaction_manager::new_transaction_manager::{
    NewTransactionManager, PsqlTransactionManager,
};
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

    // 本来は DI コンテナなどに注入して、コンテナから resolve/provide して使う想定
    let todo_repository = TodoRepositoryImpl::new();

    // すべての操作をTransaction内で実行
    let crud_result: AnyhowResult<Todo> = db_context
        .transaction(|tx| {
            Box::pin(async move {
                // 1. Create Todo
                let new_todo = Todo::new(todo_id, original_description.clone());
                let created_todo = todo_repository.create_todo(tx, new_todo).await?;
                println!("1. Created todo: {:?}", created_todo);

                // 2. Select Todo
                let found_todo = todo_repository.find_todo_by_id(tx, todo_id).await?;
                println!("2. Found todo: {:?}", found_todo);

                // 3. Update Todo
                let updated_todo = Todo::new(todo_id, updated_description.clone());
                let updated_todo = todo_repository.update_todo(tx, updated_todo).await?;
                println!("3. Updated todo: {:?}", updated_todo);

                // 4. Select updated Todo
                let final_todo = todo_repository.find_todo_by_id(tx, todo_id).await?;
                println!("4. Final todo: {:?}", final_todo);

                Ok(updated_todo)

                // let new_todo = Todo::new(todo_id, original_description.clone());
                // todo_repository
                //     .create_todo2(&mut tx, new_todo.clone())
                //     .await?;
                // Ok(new_todo)
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

    println!("\n=== 新しいTransaction Manager実証 ===");

    // 新しいTransaction Manager でのパターン実証
    let new_todo_repository = NewTodoRepositoryImpl::new();
    let new_txn_mgr = PsqlTransactionManager::new(std::sync::Arc::new(pool.clone()));

    let new_todo_id = uuid::Uuid::new_v4();
    let new_description = "New Transaction Manager Test".to_string();

    println!("--- パターン1: Transaction使用 ---");

    // Transaction開始
    new_txn_mgr.begin().await?;

    // Transaction内での複数操作
    let new_todo = Todo::new(new_todo_id, new_description.clone());
    let created = new_todo_repository
        .create_todo(&new_txn_mgr, new_todo)
        .await?;
    println!("Transaction内でTodo作成: {:?}", created);

    let found = new_todo_repository
        .find_todo_by_id(&new_txn_mgr, new_todo_id)
        .await?;
    println!("Transaction内でTodo検索: {:?}", found);

    // Commit
    new_txn_mgr.commit().await?;
    println!("Transaction commit完了");

    println!("--- パターン2: 単発Connection使用 ---");

    let single_todo_id = uuid::Uuid::new_v4();
    let single_todo = Todo::new(single_todo_id, "Single Connection Test".to_string());

    // Transaction未開始のまま単発実行
    let single_created = new_todo_repository
        .create_todo(&new_txn_mgr, single_todo)
        .await?;
    println!("単発Connection でTodo作成: {:?}", single_created);

    let single_found = new_todo_repository
        .find_todo_by_id(&new_txn_mgr, single_todo_id)
        .await?;
    println!("単発Connection でTodo検索: {:?}", single_found);

    // クリーンアップ
    let _ = query!(r#"DELETE FROM todo WHERE id = $1"#, new_todo_id)
        .execute(&pool)
        .await?;
    let _ = query!(r#"DELETE FROM todo WHERE id = $1"#, single_todo_id)
        .execute(&pool)
        .await?;

    println!("\n✅ 新しいTransaction Manager パターン完成！");
    println!("✅ Transaction/Connection 自動切り替え");
    println!("✅ 統一インターフェース");

    Ok(())
}
