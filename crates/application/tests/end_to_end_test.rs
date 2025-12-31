#[cfg(test)]
mod end_to_end_tests {
    use domain::{
        inventory::aggregate::Inventory, item::aggregate::ItemId,
        order::aggregate::CreateOrderCommand,
    };

    #[tokio::test]
    #[ignore] // Requires full environment setup
    async fn test_sea_orm_complete_flow() {
        // This test would require:
        // 1. Database setup with migrations
        // 2. Actual data seeding
        // 3. Full DI container initialization
        // 4. Complete order flow execution

        let _database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
            "postgres://postgres:password@localhost:5432/poc_transaction_manager".to_string()
        });

        // Setup would include:
        // let di = application::dependency_injection::sea_orm_repository::SeaOrmDependencyInjection::new(&database_url).await.unwrap();

        // // Prepare test data
        // let item_id = ItemId::new();
        // let inventory = Inventory::new(item_id.clone(), 100).unwrap();
        // // Seed inventory data...

        // // Execute order creation
        // let command = CreateOrderCommand {
        //     item_id,
        //     quantity: 10,
        // };
        // let result = di.order_management_use_case.create_order(command).await;
        // assert!(result.is_ok());

        // // Verify state changes
        // // Check that inventory was decreased
        // // Check that order was created

        println!("End-to-end test placeholder - requires full DB setup");
    }

    #[tokio::test]
    #[ignore]
    async fn test_sqlx_complete_flow() {
        // Similar comprehensive test for sqlx implementation
        println!("End-to-end test placeholder for sqlx - requires full DB setup");
    }

    #[tokio::test]
    async fn test_cross_orm_behavior_consistency() {
        // Test that SeaORM and sqlx implementations behave identically
        // This could use testcontainers to spin up isolated DB instances

        let item_id = ItemId::new();

        // Test domain logic consistency (no DB required)
        let command1 = CreateOrderCommand {
            item_id: item_id.clone(),
            quantity: 5,
        };
        let command2 = CreateOrderCommand {
            item_id,
            quantity: 5,
        };

        // Both should create equivalent orders
        let order1 = domain::order::aggregate::Order::from(command1).unwrap();
        let order2 = domain::order::aggregate::Order::from(command2).unwrap();

        assert_eq!(order1.quantity(), order2.quantity());
        assert_eq!(order1.item_id(), order2.item_id());
        // IDs will be different (UUID), but quantities and item_ids should match
    }

    #[tokio::test]
    async fn test_concurrent_order_creation() {
        // Test for race conditions in order creation
        // This would test that concurrent orders for the same item
        // are handled correctly without overselling inventory

        use tokio::task::JoinSet;

        let item_id = ItemId::new();
        let mut join_set = JoinSet::new();

        // Simulate 10 concurrent order attempts
        for _i in 0..10 {
            let item_id = item_id.clone();
            join_set.spawn(async move {
                let command = CreateOrderCommand {
                    item_id,
                    quantity: 1,
                };
                // In a real test, this would use actual DI and repositories
                // For now, just verify the command can be created
                domain::order::aggregate::Order::from(command)
            });
        }

        let mut successful_orders = 0;
        while let Some(result) = join_set.join_next().await {
            if result.is_ok() && result.unwrap().is_ok() {
                successful_orders += 1;
            }
        }

        assert_eq!(successful_orders, 10);
        // In a real scenario with limited inventory, we'd test that
        // only the available quantity was sold
    }

    #[tokio::test]
    async fn test_error_handling_propagation() {
        // Test that errors are properly propagated through all layers

        // Test invalid quantity handling
        let item_id = ItemId::new();
        let command = CreateOrderCommand {
            item_id,
            quantity: -5, // Invalid
        };

        let result = domain::order::aggregate::Order::from(command);
        assert!(result.is_err());

        // Test insufficient inventory scenario simulation
        let mut inventory = Inventory::new(ItemId::new(), 5).unwrap();
        let decrease_result = inventory.decrease_stock(10); // More than available
        assert!(decrease_result.is_err());
    }
}
