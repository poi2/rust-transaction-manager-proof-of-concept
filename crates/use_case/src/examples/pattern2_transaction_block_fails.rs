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
/// ## 実際のコード例とエラー
///
/// 以下のような、Transaction blockパターンを`&mut DbContext`で実装しようとすると、
/// ライフタイムエラーが発生します：
///
/// ```ignore
/// async fn create_order(&self, command: CreateOrderCommand) -> Result<Order, String> {
///     let order = Order::from(command).unwrap();
///
///     let created_order = self
///         .transaction_manager
///         .transaction(|db_context| {
///             Box::pin(async move {
///                 // 在庫を取得（排他ロックで同時更新を防止）
///                 let mut inventory = self
///                     .inventory_repository
///                     .find_by_item_id_for_update(db_context, order.item_id())
///                     .await?;
///
///                 // 在庫を更新
///                 self.inventory_repository
///                     .update(db_context, inventory)
///                     .await?;
///
///                 // 注文を保存
///                 let created_order = self
///                     .order_repository
///                     .create(db_context, order)
///                     .await?;
///
///                 Ok(created_order)
///             })
///         })
///         .await?;
///
///     Ok(created_order)
/// }
/// ```
///
/// ## 発生するエラー
///
/// ```text
/// error: lifetime may not live long enough
///   --> crates/use_case/src/examples/pattern2_transaction_block_fails.rs:99:21
///    |
/// 93 |           async fn create_order(&self, command: CreateOrderCommand) -> Result<Order, String> {
///    |                                 - let's call the lifetime of this reference `'1`
/// ...
/// 99 | /                     Box::pin(async move {
///    | |_____________________^
///    | |
///    | returning this value requires that `'1` must outlive `'static`
/// ```
///
/// ## なぜこのエラーが発生するのか
///
/// - `async move`ブロック内で`db_context: &mut DbContext`を使用しようとする
/// - `db_context`のライフタイムは`transaction`メソッドのクロージャ引数に束縛されている
/// - しかし`async move`ブロックは`'static`なFutureを要求する（Sendの要件）
/// - クロージャの引数`db_context`の参照を`async move`ブロックに移動できない
/// - **根本的に、この設計では`&mut`の借用を複数のRepository呼び出しで共有できない**
///
/// ## 解決策
///
/// この問題を解決するのが`Arc<Mutex<DbContext>>`パターンです：
///
/// - `Arc`: 複数の所有者で共有可能（'staticライフタイムを満たせる）
/// - `Mutex`: 排他制御により安全な可変アクセス
/// - Repository内で`lock().await`して一時的に可変借用を取得
/// - 使用後は自動的にロック解放
///
/// これにより、コンパイル時の安全性を保ちながら、
/// Transaction Blockパターンの利点を享受できます。
pub struct TransactionBlockPattern;

#[cfg(feature = "intentional_compile_error")]
mod compile_error_example {
    use std::sync::Arc;

    use domain::{
        inventory::aggregate::Inventory,
        item::aggregate::ItemId,
        order::aggregate::{CreateOrderCommand, Order},
    };

    struct DbContext;

    trait TransactionManager {
        async fn transaction<F, T>(&self, f: F) -> Result<T, String>
        where
            F: for<'a> FnOnce(
                    &'a mut DbContext,
                ) -> std::pin::Pin<
                    Box<dyn std::future::Future<Output = Result<T, String>> + Send + 'a>,
                > + Send;
    }

    trait InventoryRepository {
        fn find_by_item_id_for_update(
            &self,
            db_context: &mut DbContext,
            item_id: &ItemId,
        ) -> impl std::future::Future<Output = Result<Option<Inventory>, String>> + Send;

        fn update(
            &self,
            db_context: &mut DbContext,
            inventory: Inventory,
        ) -> impl std::future::Future<Output = Result<(), String>> + Send;
    }

    trait OrderRepository {
        fn create(
            &self,
            db_context: &mut DbContext,
            order: Order,
        ) -> impl std::future::Future<Output = Result<Order, String>> + Send;
    }

    struct OrderManagementUseCase<TM, IR, OR> {
        transaction_manager: Arc<TM>,
        inventory_repository: Arc<IR>,
        order_repository: Arc<OR>,
    }

    impl<TM, IR, OR> OrderManagementUseCase<TM, IR, OR>
    where
        TM: TransactionManager + Send + Sync,
        IR: InventoryRepository + Send + Sync,
        OR: OrderRepository + Send + Sync,
    {
        async fn create_order(&self, command: CreateOrderCommand) -> Result<Order, String> {
            let order = Order::from(command).unwrap();

            let created_order = self
                .transaction_manager
                .transaction(|db_context| {
                    Box::pin(async move {
                        // 在庫を取得（排他ロックで同時更新を防止）
                        let mut inventory = self
                            .inventory_repository
                            .find_by_item_id_for_update(db_context, order.item_id())
                            .await
                            .unwrap()
                            .unwrap();

                        // 注文分の在庫を減らす
                        inventory.decrease_stock(order.quantity()).unwrap();

                        // 在庫を更新
                        self.inventory_repository
                            .update(db_context, inventory)
                            .await
                            .unwrap();

                        // 注文を保存
                        let created_order = self
                            .order_repository
                            .create(db_context, order)
                            .await
                            .unwrap();

                        Ok(created_order)
                    })
                })
                .await
                .unwrap();

            Ok(created_order)
        }
    }
}
