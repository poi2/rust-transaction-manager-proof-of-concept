```rs
pub struct TransactionBlockPattern;

use std::future::Future;

use domain::{
    inventory::aggregate::Inventory,
    item::aggregate::ItemId,
    order::aggregate::{CreateOrderCommand, Order},
};

// トランザクションブロックを提供するtrait
trait TransactionManager {
    type DbContext: Send + Sync;
    type Error: Send + Sync + 'static;

    fn transaction<T, F, Fut>(&self, f: F) -> impl Future<Output = Result<T, Self::Error>> + Send
    where
        F: FnOnce(&mut Self::DbContext) -> Fut + Send,
        Fut: Future<Output = Result<T, Self::Error>> + Send,
        T: Send;
}

// Repositoryは&mut DbContextを受け取る
trait InventoryRepository {
    type DbContext;
    type Error;

    async fn find_by_item_id_for_update(
        &self,
        db_context: &mut Self::DbContext,
        item_id: &ItemId,
    ) -> Result<Option<Inventory>, Self::Error>;

    async fn update(
        &self,
        db_context: &mut Self::DbContext,
        inventory: Inventory,
    ) -> Result<(), Self::Error>;
}

trait OrderRepository {
    type DbContext;
    type Error;

    async fn create(
        &self,
        db_context: &mut Self::DbContext,
        order: Order,
    ) -> Result<Order, Self::Error>;
}

struct OrderManagementUseCase<TM, IR, OR> {
    transaction_manager: TM,
    inventory_repository: IR,
    order_repository: OR,
}

impl<TM, IR, OR> OrderManagementUseCase<TM, IR, OR>
where
    TM: TransactionManager + Send + Sync,
    IR: InventoryRepository<DbContext = TM::DbContext> + Send + Sync,
    OR: OrderRepository<DbContext = TM::DbContext> + Send + Sync,
{
    async fn create_order(&self, command: CreateOrderCommand) -> Result<Order, String> {
        let order = Order::from(command).unwrap();

        let created_order = self
            .transaction_manager
            .transaction(|db_context| {
                async move {
                    // 在庫を取得（排他ロックで同時更新を防止）
                    let mut inventory = self
                        .inventory_repository
                        .find_by_item_id_for_update(db_context, order.item_id())
                        .await
                        .map_err(|_| "Failed to find inventory")
                        .unwrap()
                        .unwrap();

                    // 注文分の在庫を減らす
                    inventory.decrease_stock(order.quantity()).unwrap();

                    // 在庫を更新
                    self.inventory_repository
                        .update(db_context, inventory)
                        .await
                        .map_err(|_| "Failed to update inventory")
                        .unwrap();

                    // 注文を保存
                    let created_order = self
                        .order_repository
                        .create(db_context, order)
                        .await
                        .map_err(|_| "Failed to create order")
                        .unwrap();

                    Ok(created_order)
                }
            })
            .await
            .map_err(|_| "Failed to create order")
            .unwrap();

        Ok(created_order)
    }
}
```