#[cfg(test)]
mod order_management_integration_tests {
    use std::sync::Arc;

    use domain::{
        db_context::DbContext, inventory::aggregate::Inventory, item::aggregate::ItemId,
        order::aggregate::CreateOrderCommand, transaction_manager::TransactionManager,
    };
    use tokio::sync::Mutex;
    use use_case::order_management::OrderManagementUseCase;

    // Enhanced mock implementations with state tracking
    struct StatefulMockDbContext {
        tx: (),
    }
    impl DbContext for StatefulMockDbContext {
        type Tx = ();
        type Error = anyhow::Error;

        fn get_transaction(&mut self) -> &mut Self::Tx {
            &mut self.tx
        }

        async fn commit(self) -> Result<(), Self::Error> {
            Ok(())
        }

        async fn rollback(self) -> Result<(), Self::Error> {
            Ok(())
        }
    }

    struct StatefulMockTransactionManager {
        should_fail: bool,
    }

    impl StatefulMockTransactionManager {
        fn new(should_fail: bool) -> Self {
            Self { should_fail }
        }
    }

    impl TransactionManager for StatefulMockTransactionManager {
        type DbContext = StatefulMockDbContext;
        type Error = anyhow::Error;

        async fn transaction<T, F, Fut>(&self, f: F) -> Result<T, Self::Error>
        where
            F: FnOnce(Arc<Mutex<Self::DbContext>>) -> Fut + Send,
            Fut: std::future::Future<Output = Result<T, Self::Error>> + Send,
            T: Send,
        {
            if self.should_fail {
                return Err(anyhow::anyhow!("Transaction failed intentionally"));
            }

            let db_context = Arc::new(Mutex::new(StatefulMockDbContext { tx: () }));
            f(db_context).await
        }
    }

    struct StatefulMockInventoryRepository {
        inventory: Arc<Mutex<Option<Inventory>>>,
        should_fail_update: bool,
    }

    impl StatefulMockInventoryRepository {
        fn new(inventory: Option<Inventory>, should_fail_update: bool) -> Self {
            Self {
                inventory: Arc::new(Mutex::new(inventory)),
                should_fail_update,
            }
        }
    }

    impl domain::inventory::repository::InventoryRepository for StatefulMockInventoryRepository {
        type DbContext = StatefulMockDbContext;
        type Error = anyhow::Error;

        async fn find_by_item_id_for_update(
            &self,
            _db_context: &Arc<Mutex<Self::DbContext>>,
            _item_id: &ItemId,
        ) -> Result<Option<Inventory>, Self::Error> {
            let inventory = self.inventory.lock().await;
            Ok(inventory.clone())
        }

        async fn find_by_item_id(
            &self,
            _db_context: &Arc<Mutex<Self::DbContext>>,
            _item_id: &ItemId,
        ) -> Result<Option<Inventory>, Self::Error> {
            let inventory = self.inventory.lock().await;
            Ok(inventory.clone())
        }

        async fn create(
            &self,
            _db_context: &Arc<Mutex<Self::DbContext>>,
            inventory: Inventory,
        ) -> Result<Inventory, Self::Error> {
            let mut stored_inventory = self.inventory.lock().await;
            *stored_inventory = Some(inventory.clone());
            Ok(inventory)
        }

        async fn update(
            &self,
            _db_context: &Arc<Mutex<Self::DbContext>>,
            inventory: Inventory,
        ) -> Result<Inventory, Self::Error> {
            if self.should_fail_update {
                return Err(anyhow::anyhow!("Update failed intentionally"));
            }

            let mut stored_inventory = self.inventory.lock().await;
            *stored_inventory = Some(inventory.clone());
            Ok(inventory)
        }
    }

    struct StatefulMockOrderRepository {
        should_fail_create: bool,
    }

    impl StatefulMockOrderRepository {
        fn new(should_fail_create: bool) -> Self {
            Self { should_fail_create }
        }
    }

    impl domain::order::repository::OrderRepository for StatefulMockOrderRepository {
        type DbContext = StatefulMockDbContext;
        type Error = anyhow::Error;

        async fn find_by_id(
            &self,
            _db_context: &Arc<Mutex<Self::DbContext>>,
            _id: &domain::order::aggregate::OrderId,
        ) -> Result<Option<domain::order::aggregate::Order>, Self::Error> {
            Ok(None)
        }

        async fn create(
            &self,
            _db_context: &Arc<Mutex<Self::DbContext>>,
            order: domain::order::aggregate::Order,
        ) -> Result<domain::order::aggregate::Order, Self::Error> {
            if self.should_fail_create {
                return Err(anyhow::anyhow!("Order creation failed intentionally"));
            }
            Ok(order)
        }
    }

    #[tokio::test]
    async fn test_successful_order_creation_flow() {
        let item_id = ItemId::new();
        let inventory = Inventory::new(item_id.clone(), 10).unwrap();

        let use_case = OrderManagementUseCase::new(
            Arc::new(StatefulMockTransactionManager::new(false)),
            Arc::new(StatefulMockInventoryRepository::new(Some(inventory), false)),
            Arc::new(StatefulMockOrderRepository::new(false)),
        );

        let command = CreateOrderCommand {
            item_id,
            quantity: 3,
        };

        let result = use_case.create_order(command).await;
        assert!(result.is_ok());
        let order = result.unwrap();
        assert_eq!(order.quantity().value(), 3);
    }

    #[tokio::test]
    async fn test_insufficient_inventory_failure() {
        let item_id = ItemId::new();
        let inventory = Inventory::new(item_id.clone(), 2).unwrap(); // Only 2 in stock

        let use_case = OrderManagementUseCase::new(
            Arc::new(StatefulMockTransactionManager::new(false)),
            Arc::new(StatefulMockInventoryRepository::new(Some(inventory), false)),
            Arc::new(StatefulMockOrderRepository::new(false)),
        );

        let command = CreateOrderCommand {
            item_id,
            quantity: 5, // Requesting more than available
        };

        let result = use_case.create_order(command).await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Insufficient stock")
        );
    }

    #[tokio::test]
    async fn test_inventory_not_found_failure() {
        let item_id = ItemId::new();

        let use_case = OrderManagementUseCase::new(
            Arc::new(StatefulMockTransactionManager::new(false)),
            Arc::new(StatefulMockInventoryRepository::new(None, false)), // No inventory
            Arc::new(StatefulMockOrderRepository::new(false)),
        );

        let command = CreateOrderCommand {
            item_id,
            quantity: 3,
        };

        let result = use_case.create_order(command).await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Inventory not found")
        );
    }

    #[tokio::test]
    async fn test_transaction_rollback_on_order_creation_failure() {
        let item_id = ItemId::new();
        let inventory = Inventory::new(item_id.clone(), 10).unwrap();

        let use_case = OrderManagementUseCase::new(
            Arc::new(StatefulMockTransactionManager::new(false)),
            Arc::new(StatefulMockInventoryRepository::new(Some(inventory), false)),
            Arc::new(StatefulMockOrderRepository::new(true)), // Order creation will fail
        );

        let command = CreateOrderCommand {
            item_id,
            quantity: 3,
        };

        let result = use_case.create_order(command).await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Order creation failed")
        );
    }

    #[tokio::test]
    async fn test_transaction_manager_failure() {
        let item_id = ItemId::new();
        let inventory = Inventory::new(item_id.clone(), 10).unwrap();

        let use_case = OrderManagementUseCase::new(
            Arc::new(StatefulMockTransactionManager::new(true)), // Transaction will fail
            Arc::new(StatefulMockInventoryRepository::new(Some(inventory), false)),
            Arc::new(StatefulMockOrderRepository::new(false)),
        );

        let command = CreateOrderCommand {
            item_id,
            quantity: 3,
        };

        let result = use_case.create_order(command).await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Transaction failed")
        );
    }

    #[tokio::test]
    async fn test_inventory_update_failure() {
        let item_id = ItemId::new();
        let inventory = Inventory::new(item_id.clone(), 10).unwrap();

        let use_case = OrderManagementUseCase::new(
            Arc::new(StatefulMockTransactionManager::new(false)),
            Arc::new(StatefulMockInventoryRepository::new(Some(inventory), true)), // Update will fail
            Arc::new(StatefulMockOrderRepository::new(false)),
        );

        let command = CreateOrderCommand {
            item_id,
            quantity: 3,
        };

        let result = use_case.create_order(command).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Update failed"));
    }

    #[tokio::test]
    async fn test_edge_case_zero_quantity_order() {
        let item_id = ItemId::new();

        let use_case = OrderManagementUseCase::new(
            Arc::new(StatefulMockTransactionManager::new(false)),
            Arc::new(StatefulMockInventoryRepository::new(None, false)),
            Arc::new(StatefulMockOrderRepository::new(false)),
        );

        let command = CreateOrderCommand {
            item_id,
            quantity: 0, // Invalid quantity
        };

        let result = use_case.create_order(command).await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Quantity must be positive")
        );
    }

    #[tokio::test]
    async fn test_edge_case_negative_quantity_order() {
        let item_id = ItemId::new();

        let use_case = OrderManagementUseCase::new(
            Arc::new(StatefulMockTransactionManager::new(false)),
            Arc::new(StatefulMockInventoryRepository::new(None, false)),
            Arc::new(StatefulMockOrderRepository::new(false)),
        );

        let command = CreateOrderCommand {
            item_id,
            quantity: -1, // Invalid quantity
        };

        let result = use_case.create_order(command).await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Quantity must be positive")
        );
    }
}
