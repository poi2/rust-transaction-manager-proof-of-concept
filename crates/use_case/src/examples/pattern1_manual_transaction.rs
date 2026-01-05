/// # パターン1: 明示的なトランザクション管理
///
/// このパターンでは、UseCase内でbegin/commit/rollbackを明示的に呼び出します。
///
/// ## コンパイル結果
///
/// ✅ **コンパイル成功** - Rustの借用チェッカーは問題を検出しません
///
/// ## 問題点
///
/// ❌ **commit/rollback忘れのリスク**
/// - 開発者がcommitを忘れると変更が失われる
/// - rollbackを忘れるとリソースがリークする可能性
///
/// ❌ **エラー時の自動rollbackなし**
/// - `?`演算子で早期リターンした場合、rollbackが実行されない
/// - データ不整合の原因となる
///
/// ❌ **ボイラープレートコードの増加**
/// - すべてのUseCaseでbegin/commit/rollbackを書く必要がある
/// - コピペによるバグの温床
///
/// ## コード例
///
/// ```rust
/// use domain::{
///     db_context::DbContext,
///     inventory::aggregate::Inventory,
///     item::aggregate::ItemId,
///     order::aggregate::{CreateOrderCommand, Order},
/// };
///
/// // トランザクション管理を提供するtrait
/// trait TransactionManager {
///     type DbContext: DbContext;
///     type Error;
///
///     async fn begin(&self) -> Result<Self::DbContext, Self::Error>;
/// }
///
/// // Repositoryは&mut DbContextを受け取る
/// trait InventoryRepository {
///     type DbContext: DbContext;
///     type Error;
///
///     async fn find_by_item_id_for_update(
///         &self,
///         db_context: &mut Self::DbContext,
///         item_id: &ItemId,
///     ) -> Result<Option<Inventory>, Self::Error>;
///
///     async fn update(
///         &self,
///         db_context: &mut Self::DbContext,
///         inventory: Inventory,
///     ) -> Result<(), Self::Error>;
/// }
///
/// trait OrderRepository {
///     type DbContext: DbContext;
///     type Error;
///
///     async fn create(
///         &self,
///         db_context: &mut Self::DbContext,
///         order: Order,
///     ) -> Result<Order, Self::Error>;
/// }
///
/// // UseCase実装
/// struct OrderManagementUseCase<TM, IR, OR> {
///     transaction_manager: TM,
///     inventory_repository: IR,
///     order_repository: OR,
/// }
///
/// impl<TM, IR, OR> OrderManagementUseCase<TM, IR, OR>
/// where
///     TM: TransactionManager,
///     IR: InventoryRepository<DbContext = TM::DbContext>,
///     OR: OrderRepository<DbContext = TM::DbContext>,
/// {
///     async fn create_order(&self, command: CreateOrderCommand) -> Result<Order, String> {
///         // トランザクション開始
///         let mut db_context = self.transaction_manager.begin().await
///             .map_err(|_| "Failed to begin transaction")?;
///
///         let order = Order::try_from(command).map_err(|e| e.to_string())?;
///
///         // 在庫を取得（排他ロック）
///         let mut inventory = self
///             .inventory_repository
///             .find_by_item_id_for_update(&mut db_context, order.item_id())
///             .await
///             .map_err(|_| "Failed to find inventory")?
///             .ok_or("Inventory not found")?;
///
///         // ドメインロジック：在庫減少
///         inventory.decrease_stock(order.quantity())
///             .map_err(|e| e.to_string())?;
///
///         // 在庫更新
///         self.inventory_repository
///             .update(&mut db_context, inventory)
///             .await
///             .map_err(|_| "Failed to update inventory")?;
///
///         // 注文作成
///         let created_order = self
///             .order_repository
///             .create(&mut db_context, order)
///             .await
///             .map_err(|_| "Failed to create order")?;
///
///         // ⚠️ 問題1: commitを忘れると変更が失われる
///         db_context.commit().await
///             .map_err(|_| "Failed to commit transaction")?;
///
///         // ⚠️ 問題2: 途中でエラーが発生した場合（?演算子）、
///         //          rollbackが実行されずにトランザクションが放置される
///
///         Ok(created_order)
///     }
/// }
/// ```
///
/// ## この問題を解決するには
///
/// パターン2（Transaction Block）に進化する必要があります。
/// しかし、パターン2には別の課題（Rustの借用チェッカー）が待ち受けています。
pub struct ManualTransactionPattern;
