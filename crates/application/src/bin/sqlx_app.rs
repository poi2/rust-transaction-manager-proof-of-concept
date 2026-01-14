use std::env;

use application::ApplicationContainer;
use domain::{item::ItemId, order::CreateOrderCommand};

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let database_url = env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://postgres:password@localhost:5432/poc_transaction_manager".to_string()
    });

    println!("Starting sqlx application...");
    println!("Database URL: {database_url}");

    let app = ApplicationContainer::new_with_sqlx(&database_url).await?;

    // デモ用の注文作成
    let item_id = ItemId::new();
    let command = CreateOrderCommand {
        item_id: item_id.clone(),
        quantity: 3,
    };

    println!(
        "Creating order for item: {}, quantity: {}",
        item_id, command.quantity
    );

    match app.order_management_use_case.create_order(command).await {
        Ok(order) => {
            println!("Order created successfully: {order:?}");
            Ok(())
        }
        Err(e) => {
            eprintln!("Failed to create order: {e}");
            Err(e)
        }
    }
}
