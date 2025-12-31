#[cfg(test)]
mod repository_integration_tests {
    use std::sync::Arc;

    use domain::{
        inventory::{aggregate::Inventory, repository::InventoryRepository},
        item::aggregate::ItemId,
        order::{
            aggregate::{Order, OrderId},
            repository::OrderRepository,
        },
        transaction_manager::TransactionManager,
    };

    #[cfg(feature = "sea-orm-impl")]
    mod sea_orm_tests {
        use super::*;
        use infrastructure::repository::sea_orm_impl::*;
        use sea_orm::{Database, DatabaseConnection};

        async fn setup_sea_orm_db() -> DatabaseConnection {
            let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
                "postgres://postgres:password@localhost:5432/poc_transaction_manager".to_string()
            });

            Database::connect(database_url)
                .await
                .expect("Failed to connect to database for testing")
        }

        #[tokio::test]
        #[ignore] // Requires running database
        async fn test_sea_orm_inventory_crud() {
            let _db = setup_sea_orm_db().await;
            let transaction_manager = SeaOrmTransactionManager::new(
                "postgres://postgres:password@localhost:5432/poc_transaction_manager",
            )
            .await
            .unwrap();

            let repo = Arc::new(SeaOrmInventoryRepository);
            let item_id = ItemId::new();
            let inventory = Inventory::new(item_id.clone(), 100).unwrap();

            // Test transaction workflow
            let result = transaction_manager
                .transaction(|db_context| {
                    let repo = Arc::clone(&repo);
                    let inventory = inventory.clone();
                    async move {
                        // Create
                        let created = repo.create(&db_context, inventory).await?;

                        // Read
                        let found = repo.find_by_item_id(&db_context, &item_id).await?;
                        assert!(found.is_some());
                        assert_eq!(found.as_ref().unwrap().quantity(), 100);

                        // Update
                        let mut updated_inventory = found.unwrap();
                        updated_inventory.decrease_stock(30).unwrap();
                        let updated = repo.update(&db_context, updated_inventory).await?;
                        assert_eq!(updated.quantity(), 70);

                        Ok(created)
                    }
                })
                .await;

            assert!(result.is_ok());
        }

        #[tokio::test]
        #[ignore]
        async fn test_sea_orm_order_crud() {
            let _db = setup_sea_orm_db().await;
            let transaction_manager = SeaOrmTransactionManager::new(
                "postgres://postgres:password@localhost:5432/poc_transaction_manager",
            )
            .await
            .unwrap();

            let repo = Arc::new(SeaOrmOrderRepository);
            let order_id = OrderId::new();
            let item_id = ItemId::new();
            let order = Order::new(order_id.clone(), item_id, 5).unwrap();

            let result = transaction_manager
                .transaction(|db_context| {
                    let repo = Arc::clone(&repo);
                    let order = order.clone();
                    async move {
                        // Create
                        let created = repo.create(&db_context, order).await?;

                        // Read
                        let found = repo.find_by_id(&db_context, &order_id).await?;
                        assert!(found.is_some());
                        assert_eq!(found.as_ref().unwrap().quantity(), 5);

                        Ok(created)
                    }
                })
                .await;

            assert!(result.is_ok());
        }
    }

    #[cfg(feature = "sqlx-impl")]
    mod sqlx_tests {
        use super::*;
        use infrastructure::repository::sqlx_impl::{
            transaction_manager_v1::SqlxTransactionManagerV1, *,
        };
        use sqlx::PgPool;

        async fn setup_sqlx_db() -> PgPool {
            let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
                "postgres://postgres:password@localhost:5432/poc_transaction_manager".to_string()
            });

            PgPool::connect(&database_url)
                .await
                .expect("Failed to connect to database for testing")
        }

        #[tokio::test]
        #[ignore] // Requires running database
        async fn test_sqlx_inventory_crud() {
            let pool = setup_sqlx_db().await;
            let transaction_manager = SqlxTransactionManagerV1::new(pool);

            let repo = Arc::new(SqlxInventoryRepository);
            let item_id = ItemId::new();
            let inventory = Inventory::new(item_id.clone(), 50).unwrap();

            let result = transaction_manager
                .transaction(|db_context| {
                    let repo = Arc::clone(&repo);
                    let inventory = inventory.clone();
                    async move {
                        // Create
                        let created = repo.create(&db_context, inventory).await?;

                        // Read
                        let found = repo.find_by_item_id(&db_context, &item_id).await?;
                        assert!(found.is_some());
                        assert_eq!(found.as_ref().unwrap().quantity(), 50);

                        Ok(created)
                    }
                })
                .await;

            assert!(result.is_ok());
        }

        #[tokio::test]
        #[ignore]
        async fn test_sqlx_order_crud() {
            let pool = setup_sqlx_db().await;
            let transaction_manager = SqlxTransactionManagerV1::new(pool);

            let repo = Arc::new(SqlxOrderRepository);
            let order_id = OrderId::new();
            let item_id = ItemId::new();
            let order = Order::new(order_id.clone(), item_id, 3).unwrap();

            let result = transaction_manager
                .transaction(|db_context| {
                    let repo = Arc::clone(&repo);
                    let order = order.clone();
                    async move {
                        // Create
                        let created = repo.create(&db_context, order).await?;

                        // Read
                        let found = repo.find_by_id(&db_context, &order_id).await?;
                        assert!(found.is_some());
                        assert_eq!(found.as_ref().unwrap().quantity(), 3);

                        Ok(created)
                    }
                })
                .await;

            assert!(result.is_ok());
        }
    }

    // Cross-ORM comparison tests
    #[tokio::test]
    async fn test_domain_logic_consistency() {
        // Test that domain logic behaves identically regardless of ORM
        let item_id = ItemId::new();
        let mut inventory1 = Inventory::new(item_id.clone(), 100).unwrap();
        let mut inventory2 = Inventory::new(item_id, 100).unwrap();

        // Both should behave identically
        inventory1.decrease_stock(30).unwrap();
        inventory2.decrease_stock(30).unwrap();

        assert_eq!(inventory1.quantity(), inventory2.quantity());
        assert_eq!(inventory1.quantity(), 70);
    }
}
