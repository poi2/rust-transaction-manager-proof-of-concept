/// # パターン2: Transaction Block（自動トランザクション管理）
///
/// パターン1の問題（commit/rollback忘れ）を解決するため、
/// トランザクション管理をTransactionManagerに委譲する設計です。
///
/// ## 利点
///
/// ✅ **commit/rollbackの自動管理**
/// - 成功時は自動commit
/// - エラー時は自動rollback
/// - 開発者がトランザクション境界を意識する必要がない
///
/// ✅ **Fail-safe設計**
/// - `?`演算子で早期リターンしても安全
/// - トランザクションのリークがない
///
/// ✅ **宣言的なコード**
/// - ビジネスロジックに集中できる
/// - ボイラープレートの削減
///
/// ## 問題点
///
/// ❌ **Rustの借用チェッカーエラー**
/// - クロージャ内で複数のRepositoryが同じトランザクションを共有できない
/// - `&mut DbContext`を2回以上借用しようとするとコンパイルエラー
///
/// ## コンパイル結果
///
/// 以下のコードは**コンパイルエラー（E0499）**になります：
///
/// ```
/// use domain::{
///     inventory::aggregate::Inventory,
///     item::aggregate::ItemId,
///     order::aggregate::{CreateOrderCommand, Order},
/// };
///
/// // トランザクションブロックを提供するtrait
/// trait TransactionManager {
///     type DbContext;
///     type Error;
///
///     async fn transaction<F, T>(&self, f: F) -> Result<T, Self::Error>
///     where
///         F: FnOnce(&mut Self::DbContext) -> Result<T, Self::Error>;
/// }
///
/// // Repositoryは&mut DbContextを受け取る
/// trait InventoryRepository {
///     type DbContext;
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
///     type DbContext;
///     type Error;
///
///     async fn create(
///         &self,
///         db_context: &mut Self::DbContext,
///         order: Order,
///     ) -> Result<Order, Self::Error>;
/// }
///
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
///         self.transaction_manager.transaction(|db_context| {
///             async move {
///                 let order = Order::from(command).map_err(|e| e.to_string())?;
///
///                 // 1回目の可変借用
///                 let mut inventory = self
///                     .inventory_repository
///                     .find_by_item_id_for_update(db_context, order.item_id())
///                     .await
///                     .map_err(|_| "Failed to find inventory")?
///                     .ok_or("Inventory not found")?;
///
///                 inventory.decrease_stock(order.quantity())
///                     .map_err(|e| e.to_string())?;
///
///                 // 2回目の可変借用
///                 self.inventory_repository
///                     .update(db_context, inventory)
///                     .await
///                     .map_err(|_| "Failed to update inventory")?;
///
///                 // 3回目の可変借用
///                 // エラー: cannot borrow `*db_context` as mutable more than once at a time
///                 let created_order = self
///                     .order_repository
///                     .create(db_context, order)
///                     .await
///                     .map_err(|_| "Failed to create order")?;
///
///                 Ok(created_order)
///             }
///         }).await
///     }
/// }
/// ```
///
/// ## 発生するエラー
///
/// ```text
/// error[E0499]: cannot borrow `*db_context` as mutable more than once at a time
///   --> src/examples/pattern2_transaction_block_fails.rs:XX:YY
///    |
/// XX |                 .find_by_item_id_for_update(db_context, order.item_id())
///    |                                             ---------- first mutable borrow occurs here
/// ...
/// XX |                 .update(db_context, inventory)
///    |                         ^^^^^^^^^^ second mutable borrow occurs here
/// ```
///
/// ## なぜこのエラーが発生するのか
///
/// - `async move`ブロック内で、クロージャの引数`db_context: &mut DbContext`を複数回使用
/// - Rustは同時に複数の可変借用を禁止している
/// - トランザクションはDBとの通信を通じて内部状態を変化させるため、可変性が必須
/// - 各Repository呼び出しは`.await`で一旦制御を返すが、ライフタイムは継続している
///
/// ## 解決策
///
/// この問題を解決するのが`Arc<Mutex<DbContext>>`パターンです：
///
/// - `Arc`: 複数の所有者で共有
/// - `Mutex`: 排他制御により安全な可変アクセス
/// - Repository内で`lock().await`して一時的に可変借用を取得
/// - 使用後は自動的にロック解放
///
/// これにより、コンパイル時の安全性を保ちながら、
/// Transaction Blockパターンの利点を享受できます。
pub struct TransactionBlockPattern;
