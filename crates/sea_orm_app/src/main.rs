use anyhow::Result;
use sea_orm_repository::todo_repository::SeaOrmTodoRepository;
use sea_orm_repository::transaction_manager::SeaOrmTransactionManager;
use sea_orm_use_case::todo_use_case::TodoUseCase;

#[tokio::main]
async fn main() -> Result<()> {
    let conn_str =
        std::env::var("DATABASE_URL").expect("Env var DATABASE_URL is required for this example.");

    // SeaORM TransactionManager生成
    let transaction_manager = SeaOrmTransactionManager::new(&conn_str).await?;
    let todo_repository = SeaOrmTodoRepository;
    let todo_use_case = TodoUseCase::new(transaction_manager, todo_repository);

    println!("=== SeaORM版のTransaction Manager使用例 (Workspace版) ===");

    // 1. 単一の操作：Todoを作成
    let created_todo = todo_use_case.create_todo("Learn SeaORM with Rust").await?;

    println!("✅ Todo作成完了: {created_todo:?}");

    // 2. 複数の操作：複数のTodoを作成し、取得する
    let descriptions = vec![
        "Implement Clean Architecture with SeaORM",
        "Master PostgreSQL with SeaORM",
    ];

    match todo_use_case
        .create_multiple_todos_and_get_all(descriptions)
        .await
    {
        Ok((created_todos, all_todos)) => {
            println!("✅ 複数操作完了:");
            for (i, todo) in created_todos.iter().enumerate() {
                println!("   作成したTodo{}: {todo:?}", i + 1);
            }
            println!("   全Todoリスト ({} 件):", all_todos.len());
            for (i, todo) in all_todos.iter().enumerate() {
                println!("     {}. {:?}", i + 1, todo);
            }
        }
        Err(e) => {
            println!("❌ エラーが発生しました: {e:?}");
        }
    }

    // 3. 検索操作：IDで特定のTodoを検索
    let found_todo = todo_use_case.find_todo_by_id(created_todo.id).await?;

    match found_todo {
        Some(todo) => println!("✅ Todo検索成功: {todo:?}"),
        None => println!("❌ Todo が見つかりませんでした"),
    }

    // 4. 更新操作のデモ
    let updated_todo = todo_use_case
        .update_todo(
            created_todo.clone(),
            "Updated: Learn SeaORM with Rust advanced features".to_string(),
        )
        .await?;

    println!("✅ Todo更新完了: {updated_todo:?}");

    println!("=== SeaORM版 処理完了 ===");
    Ok(())
}
