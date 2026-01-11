// このモジュールは意図的にコンパイルエラーを含みます。
// 通常はfeature "intentional_compile_error" でのみコンパイルされます。
//
// エラーメッセージを確認したい場合：
// 1. crates/use_case/src/lib.rs でこのモジュールのコメントを外す
// 2. `cargo check` を実行
//
// 期待されるエラー：
// error[E0499]: cannot borrow `tx` as mutable more than once at a time

use std::sync::Arc;

use domain::{db_context::DbContext, inventory::aggregate::Inventory, order::aggregate::Order};

trait TransactionManager {
    type DbContext: DbContext;
    type Error: Send + Sync + 'static;

    async fn begin(&self) -> Self::DbContext;
}

trait InventoryRepository {
    type DbContext: DbContext;
    type Error: Send + Sync + 'static;

    async fn find_by_item_id_for_update(
        &self,
        _db_context: &mut Self::DbContext,
        _item_id: &domain::item::aggregate::ItemId,
    ) -> Result<Option<Inventory>, Self::Error>;

    async fn update(
        &self,
        _db_context: &mut Self::DbContext,
        _inventory: Inventory,
    ) -> Result<(), Self::Error>;
}

trait OrderRepository {
    type DbContext: DbContext;
    type Error: Send + Sync + 'static;

    async fn create(
        &self,
        _db_context: &mut Self::DbContext,
        _order: Order,
    ) -> Result<Order, Self::Error>;
}

struct OrderManagementUseCase<TM, IR, OR> {
    transaction_manager: Arc<TM>,
    inventory_repository: Arc<IR>,
    order_repository: Arc<OR>,
}

impl<TM, IR, OR> OrderManagementUseCase<TM, IR, OR>
where
    TM: TransactionManager + Send + Sync,
    TM::Error: From<anyhow::Error>
        + From<domain::inventory::aggregate::InventoryError>
        + From<domain::order::aggregate::QuantityError>
        + From<<<TM as TransactionManager>::DbContext as DbContext>::Error>,
    IR: InventoryRepository<DbContext = TM::DbContext, Error = TM::Error>,
    OR: OrderRepository<DbContext = TM::DbContext, Error = TM::Error>,
{
    #[allow(dead_code)]
    async fn create_order(&self, order: Order) -> Result<Order, TM::Error> {
        let mut db_context = self.transaction_manager.begin().await;

        let mut inventory = self
            .inventory_repository
            .find_by_item_id_for_update(&mut db_context, order.item_id())
            .await?
            .ok_or_else(|| anyhow::anyhow!("Inventory not found for item: {}", order.item_id()))?;

        // 注文分の在庫を減らす
        inventory.decrease_stock(order.quantity())?;

        // 在庫を更新
        self.inventory_repository
            .update(&mut db_context, inventory)
            .await?;

        // 注文を作成
        let created_order = self.order_repository.create(&mut db_context, order).await?;

        db_context.commit().await?;

        Ok(created_order)
    }
}
