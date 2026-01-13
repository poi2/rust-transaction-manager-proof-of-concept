# Transaction Monad Pattern in Rust

## 概要

トランザクション処理をState Monadとして抽象化し、関数合成によって複雑なビジネスロジックを構築するパターン。
副作用を型レベルで管理し、純粋関数的なスタイルでトランザクション操作を記述できます。

## 基本設計

### Transaction Monad の定義

```rust
use std::pin::Pin;
use std::future::Future;
use sqlx::{PgPool, Transaction, Postgres};

type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

#[derive(Debug)]
pub enum TxError {
    Database(sqlx::Error),
    Business(String),
    NotFound,
}

impl From<sqlx::Error> for TxError {
    fn from(err: sqlx::Error) -> Self {
        TxError::Database(err)
    }
}

/// Transaction Monad
/// トランザクション状態を保持しながら計算を進める
pub struct TxAction<T> {
    action: Box<dyn FnOnce(Transaction<'static, Postgres>)
        -> BoxFuture<'static, Result<(T, Transaction<'static, Postgres>), TxError>> + Send>,
}

impl<T: Send + 'static> TxAction<T> {
    /// Monad の return (pure) - 値をMonadにラップ
    pub fn pure(value: T) -> Self {
        Self {
            action: Box::new(move |tx| {
                Box::pin(async move { Ok((value, tx)) })
            }),
        }
    }

    /// Monad の bind (flatMap) - Monadic合成
    pub fn bind<U: Send + 'static, F>(self, f: F) -> TxAction<U>
    where
        F: FnOnce(T) -> TxAction<U> + Send + 'static,
    {
        TxAction {
            action: Box::new(move |tx| {
                Box::pin(async move {
                    let (value, tx) = (self.action)(tx).await?;
                    let next_action = f(value);
                    (next_action.action)(tx).await
                })
            }),
        }
    }

    /// map - Functorとしての操作
    pub fn map<U: Send + 'static, F>(self, f: F) -> TxAction<U>
    where
        F: FnOnce(T) -> U + Send + 'static,
    {
        self.bind(|value| TxAction::pure(f(value)))
    }

    /// エラーハンドリング
    pub fn map_err<F>(self, f: F) -> TxAction<T>
    where
        F: FnOnce(TxError) -> TxError + Send + 'static,
    {
        TxAction {
            action: Box::new(move |tx| {
                Box::pin(async move {
                    match (self.action)(tx).await {
                        Ok(result) => Ok(result),
                        Err(err) => Err(f(err)),
                    }
                })
            }),
        }
    }
}
```

### ドメインエンティティ

```rust
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct ItemId(Uuid);

impl ItemId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

#[derive(Debug, Clone)]
pub struct Inventory {
    item_id: ItemId,
    quantity: i32,
}

impl Inventory {
    pub fn new(item_id: ItemId, quantity: i32) -> Result<Self, TxError> {
        if quantity < 0 {
            return Err(TxError::Business("Quantity cannot be negative".to_string()));
        }
        Ok(Self { item_id, quantity })
    }

    pub fn item_id(&self) -> &ItemId {
        &self.item_id
    }

    pub fn quantity(&self) -> i32 {
        self.quantity
    }

    pub fn decrease_stock(&mut self, amount: i32) -> Result<(), TxError> {
        if self.quantity < amount {
            return Err(TxError::Business("Insufficient inventory".to_string()));
        }
        self.quantity -= amount;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct Order {
    id: Uuid,
    item_id: ItemId,
    quantity: i32,
}

impl Order {
    pub fn new(item_id: ItemId, quantity: i32) -> Self {
        Self {
            id: Uuid::new_v4(),
            item_id,
            quantity,
        }
    }

    pub fn item_id(&self) -> &ItemId {
        &self.item_id
    }

    pub fn quantity(&self) -> i32 {
        self.quantity
    }
}
```

### Repository操作のMonadic定義

```rust
/// 在庫検索（排他ロック）
pub fn find_inventory_for_update(item_id: ItemId) -> TxAction<Option<Inventory>> {
    TxAction {
        action: Box::new(move |mut tx| {
            Box::pin(async move {
                let result = sqlx::query(
                    "SELECT item_id, quantity FROM inventory WHERE item_id = $1 FOR UPDATE"
                )
                .bind(item_id.as_uuid())
                .fetch_optional(&mut *tx)
                .await?;

                let inventory = match result {
                    Some(row) => {
                        let item_id = ItemId(row.try_get("item_id")?);
                        let quantity: i32 = row.try_get("quantity")?;
                        Some(Inventory::new(item_id, quantity)?)
                    }
                    None => None,
                };

                Ok((inventory, tx))
            })
        }),
    }
}

/// 在庫更新
pub fn update_inventory(inventory: Inventory) -> TxAction<()> {
    TxAction {
        action: Box::new(move |mut tx| {
            Box::pin(async move {
                sqlx::query("UPDATE inventory SET quantity = $1 WHERE item_id = $2")
                    .bind(inventory.quantity())
                    .bind(inventory.item_id().as_uuid())
                    .execute(&mut *tx)
                    .await?;

                Ok(((), tx))
            })
        }),
    }
}

/// 注文作成
pub fn create_order(order: Order) -> TxAction<Order> {
    TxAction {
        action: Box::new(move |mut tx| {
            let order_clone = order.clone();
            Box::pin(async move {
                sqlx::query("INSERT INTO orders (id, item_id, quantity) VALUES ($1, $2, $3)")
                    .bind(&order.id)
                    .bind(order.item_id().as_uuid())
                    .bind(order.quantity())
                    .execute(&mut *tx)
                    .await?;

                Ok((order_clone, tx))
            })
        }),
    }
}

/// エラーを返すMonadic操作
pub fn require_inventory(inventory_opt: Option<Inventory>) -> TxAction<Inventory> {
    match inventory_opt {
        Some(inventory) => TxAction::pure(inventory),
        None => TxAction {
            action: Box::new(move |_tx| {
                Box::pin(async move { Err(TxError::NotFound) })
            }),
        },
    }
}

/// 在庫減少のMonadic操作
pub fn decrease_inventory_stock(mut inventory: Inventory, amount: i32) -> TxAction<Inventory> {
    match inventory.decrease_stock(amount) {
        Ok(()) => TxAction::pure(inventory),
        Err(err) => TxAction {
            action: Box::new(move |_tx| {
                Box::pin(async move { Err(err) })
            }),
        },
    }
}
```

### ビジネスロジックの関数合成

```rust
/// 注文作成のビジネスロジック
/// 関数合成によって複雑な処理を段階的に構築
pub fn create_order_business_logic(item_id: ItemId, order_quantity: i32) -> TxAction<Order> {
    find_inventory_for_update(item_id.clone())
        .bind(require_inventory)
        .bind(move |inventory| decrease_inventory_stock(inventory, order_quantity))
        .bind(|updated_inventory| {
            update_inventory(updated_inventory.clone())
                .map(|_| updated_inventory)
        })
        .bind(move |_| {
            let order = Order::new(item_id, order_quantity);
            create_order(order)
        })
}

/// より複雑な例：複数商品の注文処理
pub fn create_multi_item_order(items: Vec<(ItemId, i32)>) -> TxAction<Vec<Order>> {
    // 初期値として空のベクタを返すモナド
    let init = TxAction::pure(Vec::new());

    // 各アイテムを順次処理してモナドを合成
    items.into_iter().fold(init, |acc_action, (item_id, quantity)| {
        acc_action.bind(move |mut orders| {
            create_order_business_logic(item_id, quantity)
                .map(move |order| {
                    orders.push(order);
                    orders
                })
        })
    })
}
```

### トランザクション実行器

```rust
/// Transaction Monadを実際に実行する
pub async fn execute_transaction<T>(
    pool: &PgPool,
    action: TxAction<T>,
) -> Result<T, TxError> {
    // unsafe transmutation（実際の実装では適切な型変換が必要）
    let tx = pool.begin().await?;
    let tx_static = unsafe {
        std::mem::transmute::<
            Transaction<'_, Postgres>,
            Transaction<'static, Postgres>
        >(tx)
    };

    match (action.action)(tx_static).await {
        Ok((result, tx)) => {
            tx.commit().await?;
            Ok(result)
        }
        Err(err) => {
            // ロールバックは自動的に行われる
            Err(err)
        }
    }
}
```

### 使用例

```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pool = PgPool::connect("postgresql://user:pass@localhost/db").await?;

    let item_id = ItemId::new();

    // シンプルな使用例
    let result = execute_transaction(&pool,
        create_order_business_logic(item_id.clone(), 5)
    ).await;

    match result {
        Ok(order) => println!("Order created: {:?}", order),
        Err(TxError::NotFound) => println!("Item not found"),
        Err(TxError::Business(msg)) => println!("Business error: {}", msg),
        Err(TxError::Database(err)) => println!("Database error: {}", err),
    }

    // 複数商品の注文例
    let items = vec![
        (ItemId::new(), 3),
        (ItemId::new(), 2),
        (ItemId::new(), 1),
    ];

    let multi_result = execute_transaction(&pool,
        create_multi_item_order(items)
    ).await;

    match multi_result {
        Ok(orders) => println!("Multiple orders created: {:?}", orders),
        Err(err) => println!("Failed to create orders: {:?}", err),
    }

    Ok(())
}
```

## パターンの評価

### 利点

1. **合成可能性**: 小さな操作を組み合わせて複雑な処理を構築
2. **副作用の明示**: トランザクションの副作用が型レベルで表現される
3. **テスタビリティ**: 各操作が独立してテスト可能
4. **エラーハンドリング**: モナドのエラーチェーンで一貫したエラー処理
5. **関数型プログラミング**: 純粋関数的なスタイルでコード記述

### 欠点

1. **学習コスト**: Monadの概念理解が必要
2. **実装コスト**: 大量のボイラープレート
3. **パフォーマンス**: 関数合成と動的ディスパッチのオーバーヘッド
4. **Rust慣習**: 一般的なRustコードスタイルと大きく異なる
5. **デバッグ**: スタックトレースが複雑になりがち

### 適用場面

- **高度なRustチーム**: 関数型プログラミング経験豊富
- **複雑なトランザクション合成**: 多段階の条件分岐やループ処理が多い
- **実験的プロジェクト**: 新しいパラダイムの導入余地がある
- **DSL構築**: トランザクション処理をドメイン特化言語として表現したい場合

### 実用性評価

**推奨度**: ★★☆☆☆ (限定的)

一般的なRustアプリケーションには過度に複雑。ただし、関数型プログラミングの知識があるチームや、
非常に複雑なトランザクション処理を多用するドメインでは検討価値あり。

大部分のケースでは `Arc<Mutex>` パターンの方が実用的で保守性が高い。
