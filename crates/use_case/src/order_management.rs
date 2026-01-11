use std::sync::Arc;

use domain::{
    inventory::repository::InventoryRepository,
    order::{
        aggregate::{CreateOrderCommand, Order},
        repository::OrderRepository,
    },
    transaction_manager::TransactionManager,
};

pub struct OrderManagementUseCase<TM, IR, OR> {
    transaction_manager: Arc<TM>,
    inventory_repository: Arc<IR>,
    order_repository: Arc<OR>,
}

impl<TM, IR, OR> OrderManagementUseCase<TM, IR, OR>
where
    TM: TransactionManager + Send + Sync,
    TM::Error: From<anyhow::Error>
        + From<domain::inventory::aggregate::InventoryError>
        + From<domain::order::aggregate::QuantityError>,
    IR: InventoryRepository<DbContext = TM::DbContext, Error = TM::Error>,
    OR: OrderRepository<DbContext = TM::DbContext, Error = TM::Error>,
{
    pub fn new(
        transaction_manager: Arc<TM>,
        inventory_repository: Arc<IR>,
        order_repository: Arc<OR>,
    ) -> Self {
        Self {
            transaction_manager,
            inventory_repository,
            order_repository,
        }
    }

    pub async fn create_order(&self, command: CreateOrderCommand) -> Result<Order, TM::Error> {
        let order = Order::try_from(command)?;

        let created_order = self
            .transaction_manager
            .transaction(|db_context| {
                async move {
                    // 在庫を取得（排他ロックで同時更新を防止）
                    let mut inventory = self
                        .inventory_repository
                        .find_by_item_id_for_update(&db_context, order.item_id())
                        .await?
                        .ok_or_else(|| {
                            anyhow::anyhow!("Inventory not found for item: {}", order.item_id())
                        })?;

                    // 注文分の在庫を減らす
                    inventory.decrease_stock(order.quantity())?;

                    // 在庫を更新
                    self.inventory_repository
                        .update(&db_context, inventory)
                        .await?;

                    // 注文を作成
                    let created_order = self.order_repository.create(&db_context, order).await?;

                    Ok(created_order)
                }
            })
            .await?;

        Ok(created_order)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use domain::{
        db_context::DbContext, inventory::aggregate::Inventory, item::aggregate::ItemId,
        order::aggregate::OrderId,
    };
    use tokio::sync::Mutex;

    use super::*;

    // Mock implementations for testing
    struct MockDbContext {
        tx: (),
    }
    impl DbContext for MockDbContext {
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

    struct MockTransactionManager;
    impl TransactionManager for MockTransactionManager {
        type DbContext = MockDbContext;
        type Error = anyhow::Error;

        async fn transaction<T, F, Fut>(&self, f: F) -> Result<T, Self::Error>
        where
            F: FnOnce(Arc<Mutex<Self::DbContext>>) -> Fut + Send,
            Fut: std::future::Future<Output = Result<T, Self::Error>> + Send,
            T: Send,
        {
            let db_context = Arc::new(Mutex::new(MockDbContext { tx: () }));
            f(db_context).await
        }
    }

    struct MockInventoryRepository {
        inventory: Option<Inventory>,
    }

    impl InventoryRepository for MockInventoryRepository {
        type DbContext = MockDbContext;
        type Error = anyhow::Error;

        async fn find_by_item_id_for_update(
            &self,
            _db_context: &Arc<Mutex<Self::DbContext>>,
            _item_id: &ItemId,
        ) -> Result<Option<Inventory>, Self::Error> {
            Ok(self.inventory.clone())
        }

        async fn find_by_item_id(
            &self,
            _db_context: &Arc<Mutex<Self::DbContext>>,
            _item_id: &ItemId,
        ) -> Result<Option<Inventory>, Self::Error> {
            Ok(self.inventory.clone())
        }

        async fn create(
            &self,
            _db_context: &Arc<Mutex<Self::DbContext>>,
            inventory: Inventory,
        ) -> Result<Inventory, Self::Error> {
            Ok(inventory)
        }

        async fn update(
            &self,
            _db_context: &Arc<Mutex<Self::DbContext>>,
            inventory: Inventory,
        ) -> Result<Inventory, Self::Error> {
            Ok(inventory)
        }
    }

    struct MockOrderRepository;
    impl OrderRepository for MockOrderRepository {
        type DbContext = MockDbContext;
        type Error = anyhow::Error;

        async fn find_by_id(
            &self,
            _db_context: &Arc<Mutex<Self::DbContext>>,
            _id: &OrderId,
        ) -> Result<Option<Order>, Self::Error> {
            Ok(None)
        }

        async fn create(
            &self,
            _db_context: &Arc<Mutex<Self::DbContext>>,
            order: Order,
        ) -> Result<Order, Self::Error> {
            Ok(order)
        }
    }

    #[tokio::test]
    async fn test_create_order_success() {
        let item_id = ItemId::new();
        let inventory = Inventory::new(item_id.clone(), 10).unwrap();

        let use_case = OrderManagementUseCase::new(
            Arc::new(MockTransactionManager),
            Arc::new(MockInventoryRepository {
                inventory: Some(inventory),
            }),
            Arc::new(MockOrderRepository),
        );

        let command = CreateOrderCommand {
            item_id,
            quantity: 3,
        };

        let result = use_case.create_order(command).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_create_order_inventory_not_found() {
        let item_id = ItemId::new();

        let use_case = OrderManagementUseCase::new(
            Arc::new(MockTransactionManager),
            Arc::new(MockInventoryRepository { inventory: None }),
            Arc::new(MockOrderRepository),
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
}
