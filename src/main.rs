use infrastructure::todo_repository_sqlx_mutex_guard::SqlxTodoRepositoryMutexGuard;
use infrastructure::transaction_manager::db_context::DBContext;

use domain::todo_repository_mutex_guard::{Todo, TodoRepositoryMutexGuard};
use domain::transaction_manager_mutex_guard::TransactionManagerMutexGuard;

mod domain;
mod infrastructure;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let conn_str =
        std::env::var("DATABASE_URL").expect("Env var DATABASE_URL is required for this example.");
    let pool = sqlx::PgPool::connect(&conn_str).await?;

    // DBContext生成
    let transaction_manager = DBContext::new(pool.clone());
    let todo_repository = SqlxTodoRepositoryMutexGuard;

    println!("=== MutexGuard版のTransaction Manager使用例 ===");

    // 1. 単一の操作：Todoを作成
    let todo_id1 = uuid::Uuid::new_v4();
    let created_todo = transaction_manager
        .transaction(|db_context| {
            let todo_repository = todo_repository.clone();
            async move {
                todo_repository
                    .create(&db_context, todo_id1, "Learn Rust async programming")
                    .await
            }
        })
        .await?;

    println!("✅ Todo作成完了: {:?}", created_todo);

    // 2. 複数の操作：複数のTodoを作成し、取得する
    let todo_id2 = uuid::Uuid::new_v4();
    let todo_id3 = uuid::Uuid::new_v4();

    let results: Result<(Todo, Todo, Vec<Todo>), anyhow::Error> = transaction_manager
        .transaction(|db_context| {
            let todo_repository = todo_repository.clone();
            async move {
                // 1回目の操作：Todo作成
                let todo2 = todo_repository
                    .create(&db_context, todo_id2, "Implement Clean Architecture")
                    .await?;

                // 2回目の操作：別のTodo作成
                let todo3 = todo_repository
                    .create(&db_context, todo_id3, "Master PostgreSQL")
                    .await?;

                // 3回目の操作：全Todoを取得
                let all_todos = todo_repository.find_all(&db_context).await?;

                Ok((todo2, todo3, all_todos))
            }
        })
        .await;

    match results {
        Ok((todo2, todo3, all_todos)) => {
            println!("✅ 複数操作完了:");
            println!("   作成したTodo2: {:?}", todo2);
            println!("   作成したTodo3: {:?}", todo3);
            println!("   全Todoリスト ({} 件):", all_todos.len());
            for (i, todo) in all_todos.iter().enumerate() {
                println!("     {}. {:?}", i + 1, todo);
            }
        }
        Err(e) => {
            println!("❌ エラーが発生しました: {:?}", e);
        }
    }

    // 3. 検索操作：IDで特定のTodoを検索
    let found_todo = transaction_manager
        .transaction(|db_context| {
            let todo_repository = todo_repository.clone();
            async move { todo_repository.find_by_id(&db_context, todo_id1).await }
        })
        .await?;

    match found_todo {
        Some(todo) => println!("✅ Todo検索成功: {:?}", todo),
        None => println!("❌ Todo が見つかりませんでした"),
    }

    println!("=== 処理完了 ===");
    Ok(())
}
