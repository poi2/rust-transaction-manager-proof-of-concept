Rust アプリケーションにおける実践的トランザクション設計
-----

# Rust におけるトランザクション管理の現実解

Rust でエンタープライズアプリケーションを構築する際、トランザクション管理の設計と実装が壁となります。
所有権システムの制約により、他言語では当たり前のパターンが適用できず、多くの開発者が実装に悩むポイントとなっています。

本記事では、実際のプロダクション環境で使用できる実装パターンを、具体的なコード例とともに解説します。
Rust におけるスタンダードな DB アクセスライブラリーである SeaORM と sqlx の両方での実装を通じて、実践的なアプローチを提示します。

本記事では、PostgreSQL や MySQL のようなトランザクション機能を持つ RDBMS を対象としています。

# なぜRustでトランザクション管理は困難なのか

## 理想的なトランザクション管理とは

トランザクション管理において、理想的な設計とはどのようなものでしょうか？
筆者の考えでは以下のような要件を望みます。

1. 透過的: ビジネスロジックに集中でき、トランザクション境界をコントロールできる
2. 安全性: エラー時に自動的に rollback される
3. 自動的な commit/rollback: 開発者が明示的に呼び出す必要がない

他の言語では、これらは容易に実現できます。
具体例を示すと……

Java の Spring　フレームワークでは `@Transactional` アノテーションを使ってトランザクションを管理できます。

```java
@Transactional
public void createOrder(Order order) {
    inventoryRepository.update(inventory);
    orderRepository.create(order);
} // 成功時は自動 commit、エラー時は自動 rollback
```

Ruby の Ruby on Rails フレームワークの ActiveRecord では `transaction` メソッドを使ってトランザクションを管理できます。

```ruby
ActiveRecord::Base.transaction do
  inventory.update!(...)
  order.save!(...)
end # 成功時は自動 commit、エラー時は自動 rollback
```

Rust でもこのような Transaction Block パターンを導入したいと考えるのは自然なことです。

## Rustで実装しようとすると...

Rust で同様の Transaction Block パターンを実装しようとすると、以下のようなコードになります。
具体例があると説明しやすいので、EC サイトで「注文の確定と、その商品の在庫を減らす」というユースケースを例に取りましょう。

```rust
// Transaction Manager trait
pub trait TransactionManager {
    type DbContext: DbContext;
    type Error: Send + Sync + 'static;

    fn transaction<T, F, Fut>(&self, f: F) -> impl Future<Output = Result<T, Self::Error>> + Send
    where
        F: for<'a> FnOnce(
                &'a mut DbContext,
            ) -> std::pin::Pin<
                Box<dyn std::future::Future<Output = Result<T, Self::Error>> + Send + 'a>,
            > + Send;
}

// create_order ユースケース
impl<TM, IR, OR> OrderManagementUseCase<TM, IR, OR>
where
    TM: TransactionManager + Send + Sync,
    IR: InventoryRepository + Send + Sync,
    OR: OrderRepository + Send + Sync,
{
    async fn create_order(&self, command: CreateOrderCommand) -> anyhow::Result<Order> {
        let order = Order::try_from(command).unwrap();

        let created_order = self
            .transaction_manager
            .transaction(|db_context| {
                Box::pin(async move {
                    // 在庫を取得（排他ロックで同時更新を防止）
                    let mut inventory = self
                        .inventory_repository
                        .find_by_item_id_for_update(db_context, order.item_id())
                        .await?
                        .ok_or_else(|| {
                            anyhow::anyhow!("Inventory not found for item: {}", order.item_id())
                        })?;

                    // 注文分の在庫を減らす
                    inventory.decrease_stock(order.quantity())?;

                    // 在庫を更新
                    self.inventory_repository
                        .update(db_context, inventory)
                        .await?;

                    // 注文を作成
                    let created_order = self.order_repository.create(db_context, order).await?;

                    Ok(created_order)
                })
            })
            .await
            .unwrap();

        Ok(created_order)
    }
}
```

このコードは一見正しそうに見えますが、残念ながらコンパイルエラーとなります。

```text
error: lifetime may not live long enough
   --> crates/use_case/src/examples/pattern2_transaction_block_fails.rs:164:21
    |
158 |           async fn create_order(&self, command: CreateOrderCommand) -> anyhow::Result<Order> {
    |                                 - let's call the lifetime of this reference `'1`
...
164 | /                     Box::pin(async move {
165 | |                         // 在庫を取得（排他ロックで同時更新を防止）
166 | |                         let mut inventory = self
167 | |                             .inventory_repository
...   |
185 | |                         Ok(created_order)
186 | |                     })
    | |______________________^ returning this value requires that `'1` must outlive `'static`

error: could not compile `use_case` (lib) due to 1 previous error
```

## なぜエラーが発生するのか

このエラーの根本原因は、Rust の所有権システムとライフタイムの制約にあります：

1. `async move` ブロックが `'static` な Future を要求する
    - 非同期ランタイム上ではいつ実行されるか不明なため、トランザクションオブジェクトへの参照を渡すことができない
    - つまり `async move` ブロックにすべてのデータの所有権を渡すか、`'static`（どのライフタイムにも依存しない）なデータである必要がある
2. `db_context` のライフタイムのミスマッチにより、Rust がコンパイルできない
    - `db_context` は `transaction()` が作成し所有し、`transaction()` が終わるまで存在します（終わると無効になる）
    - `transaction()` は closure に `&mut db_context` という参照を渡す
    - closure は `async move {...}` ブロックを作成して返す
    - closure 終了後、`&mut db_context` の参照は無効になる
    - しかし、`async move {...}` ブロックは closure 終了後に await で実行され、その時点では `&mut db_context` の参照は無効である
    - Rust ではこの UAF (Use-After-Free) を防ぐためにコンパイルエラーを発生させる
3. 複数の Repository の呼び出しにおいて同じ `db_context` を共有する必要がある
    - Transaction Block 内は同じトランザクションで実行されることを期待する

根本的に、closure f に `&mut DbContext` を渡す設計では、Transaction Blockパターンを実現できません。

<details>
<summary>トランザクションの手動管理は実装可能だが合理的でない理由</summary>

以下のようなトランザクションの手動管理ならコンパイルは通ります。

```rust
async fn create_order(&self, command: CreateOrderCommand) -> Result<Order, String> {
    let order = Order::try_from(command)?;
    let mut db_context = self.transaction_manager.begin().await?;

    let mut inventory = self
        .inventory_repository
        .find_by_item_id_for_update(&mut db_context, order.item_id())
        .await?
        .ok_or("Inventory not found")?;

    self.inventory_repository
        .update(&mut db_context, inventory)
        .await?;

    self.order_repository
        .create(&mut db_context, order)
        .await?;

    db_context.commit().await?; // commitを忘れると変更が失われる

    Ok(created_order)
}
```

この場合、`db_context` は関数スコープ内に所有されており、`async move` ブロックがないため、ライフタイム問題は発生しません。

しかし、この設計には重大な問題があります。

- commit を実装し忘れると DB への永続化が行われない
- `?` や early return を使うとき commit/rollback をすり抜ける

関数を抜けるすべてのパターンで正しく commit/rollback を実装することは理論上可能ですが、技術的複雑性が高く合理的ではありません。
</details>

## Rust での技術的制約

Rust でトランザクション管理を実現しつつ現実的な実装に落とし込むには、以下の制約をすべて満たす必要があります。

1. 所有権の問題
    - トランザクションオブジェクトを利用する各関数やブロックの適切な所有権を明らかにすること
2. ライフタイムの問題
    - `async move` ブロック内でトランザクションが利用でき、かつ closure のライフタイムに束縛されない設計
3. 安全なトランザクションの共有
    - 複数の Repository の間でトランザクションオブジェクトを共有しつつ、データ競合を防ぐ設計
4. 抽象化
    - Clean Architecture の依存性逆転の原則への対応
    - 上記の型パズルを満たす TransactionManager の設計
5. ORM 互換性
    - SeaORM や sqlx の crate が実装可能でかつ互換性があること

これらすべてを同時に満たす解を求める必要があります。

# 実践的解決パターン

`Arc<Mutex>` による排他的可変共有のパターンを取り入れることで「Rust での技術的制約」の 1, 2, 3 を解消します。

## `Arc<Mutex>` による共有トランザクション

### 設計概要

最も実用的で広く採用できるのが、`Arc<Mutex>` による排他的可変共有パターンです。

```rust
use std::sync::Arc;
use tokio::sync::Mutex;

// トランザクションを Arc<Mutex> でラップ
let db_context = Arc::new(Mutex::new(transaction_context));

// 複数のRepositoryで安全に共有
inventory_repository.update(&db_context, inventory).await?;
order_repository.create(&db_context, order).await?;
```

この設計により、コンパイル時安全性を保ちながらトランザクションの共有が可能になります。

### なぜ Arc<Mutex> で解決できるのか

先ほどのライフタイムエラーは、以下の理由で解決されます：

1. `Arc` で所有権を共有
   - `Arc::clone()` で複数の所有者を作れる
   - すべてのデータが所有されているため、`'static` 要件を満たせる
2. `Mutex` で安全な可変アクセス
   - `lock().await` で一時的に排他的な可変借用を取得
   - 使用後は自動的にロック解放
   - データ競合を防ぐ
   - 一方でロックの取得と解放による追加コストが生じる
3. `async move` ブロック内で使用可能
   - `Arc<Mutex<DbContext>>` 自体を move できる
   - ライフタイム制約から解放される

## 実装

ここから具体的な実装を行っていきます。

### Step 1: トランザクション抽象化の設計

まず、DB や ORM に依存しない抽象化レイヤーを定義します。

```rust
// domain/src/db_context.rs

/// Domain layer database context abstraction
#[allow(async_fn_in_trait)]
pub trait DbContext: Send + Sync {
    type Tx: Send;
    type Error: Send + Sync + 'static;

    /// Get mutable reference to the underlying database transaction
    fn get_transaction(&mut self) -> &mut Self::Tx;

    /// Commit transaction - consumes self to prevent reuse
    async fn commit(self) -> Result<(), Self::Error>;

    /// Rollback transaction - consumes self to prevent reuse
    async fn rollback(self) -> Result<(), Self::Error>;
}
```

この抽象化により、TransactionManager, Repository は DbContext を知っているが、お互いは直接的に依存がない状態を作り出します。
これにより TransactionManager, Repository の責務・関心の分離と疎結合にできるようになりました。

### Step 2: Repository traitの定義

Repository では `&Arc<Mutex<DbContext>>` を実行時に受け取る設計とします。

```rust
// domain/src/inventory/repository.rs

use std::{future::Future, sync::Arc};
use tokio::sync::Mutex;

#[allow(async_fn_in_trait)]
pub trait InventoryRepository: Send + Sync {
    type DbContext: DbContext;
    type Error: Send + Sync + 'static;

    fn find_by_item_id_for_update(
        &self,
        db_context: &Arc<Mutex<Self::DbContext>>,
        item_id: &ItemId,
    ) -> impl Future<Output = Result<Option<Inventory>, Self::Error>> + Send
    where
        Self: Send;

    fn update(
        &self,
        db_context: &Arc<Mutex<Self::DbContext>>,
        inventory: Inventory,
    ) -> impl Future<Output = Result<Inventory, Self::Error>> + Send
    where
        Self: Send;
}
```

### Step 3: TransactionManager の実装

トランザクション管理の中核となる TransactionManager の定義です。
`F: FnOnce(Arc<Mutex<Self::DbContext>>) -> Fut + Send,` の `Arc<Mutex<Self::DbContext>>` の部分が本記事の改善点です。

```rust
// domain/src/transaction_manager.rs

use std::{future::Future, sync::Arc};
use tokio::sync::Mutex;

pub trait TransactionManager {
    type DbContext: DbContext;
    type Error: Send + Sync + 'static;

    fn transaction<T, F, Fut>(&self, f: F) -> impl Future<Output = Result<T, Self::Error>> + Send
    where
        F: FnOnce(Arc<Mutex<Self::DbContext>>) -> Fut + Send,
        Fut: Future<Output = Result<T, Self::Error>> + Send,
        T: Send;
}
```

### Step 4: TransactionManager と Repository の実装

ここからは TransactionManager や Repository の実装を行っていきます。
ORM として SeaORM と sqlx を対象としています。

#### Step 4-1: SeaORM による実装

SeaORM による TransactionManager の実装です。

```rust
// infrastructure/repository/sea_orm_impl/src/transaction_manager.rs

use std::sync::Arc;
use tokio::sync::Mutex;
use sea_orm::{DatabaseConnection, TransactionTrait};

pub struct SeaOrmTransactionManager {
    db: DatabaseConnection,
}

impl TransactionManager for SeaOrmTransactionManager {
    type DbContext = SeaOrmDbContext;
    type Error = anyhow::Error;

    async fn transaction<T, F, Fut>(&self, f: F) -> Result<T, Self::Error>
    where
        F: FnOnce(Arc<Mutex<Self::DbContext>>) -> Fut + Send,
        Fut: Future<Output = Result<T, Self::Error>> + Send,
        T: Send,
    {
        let txn = self.db.begin().await?;
        let db_context = SeaOrmDbContext::new(txn);
        let db_context = Arc::new(Mutex::new(db_context));

        match f(db_context.clone()).await {
            Ok(result) => {
                // Arc::try_unwrap で安全に抽出して commit
                match Arc::try_unwrap(db_context) {
                    Ok(mutex) => {
                        let context = mutex.into_inner();
                        context.commit().await?;
                        Ok(result)
                    }
                    Err(_) => {
                        Err(anyhow::anyhow!("Failed to extract context for commit"))
                    }
                }
            }
            Err(e) => {
                // エラー時は自動 rollback
                match Arc::try_unwrap(db_context) {
                    Ok(mutex) => {
                        let context = mutex.into_inner();
                        let _ = context.rollback().await;
                    }
                    Err(_) => {
                        // ロールバック失敗は無視（通常発生しない）
                    }
                }
                Err(e)
            }
        }
    }
}
```

実際の Repository 実装では、`Arc<Mutex>` から安全にトランザクションを取得します。
本記事では InventoryRepository の SELECT FOR UPDATE しつつ Inventory を取得するメソッドだけ紹介します。

```rust
impl InventoryRepository for SeaOrmInventoryRepository {
    type DbContext = SeaOrmDbContext;
    type Error = anyhow::Error;

    async fn find_by_item_id_for_update(
        &self,
        db_context: &Arc<Mutex<Self::DbContext>>,
        item_id: &ItemId,
    ) -> Result<Option<Inventory>, Self::Error> {
        let mut guard = db_context.lock().await; // db_context を排他的ロック
        let txn = guard.get_transaction(); // ロック取得後、トランザクションを取得

        // トランザクションを使って Inventory を取得
        let result = Entity::find()
            .filter(Column::ItemId.eq(*item_id.as_uuid()))
            .lock_exclusive()
            .one(txn)
            .await?;

        // Inventory を組み立てて返す
        match result {
            Some(model) => {
                let inventory = Inventory::new(
                    ItemId::from_uuid(model.item_id),
                    model.quantity
                )?;
                Ok(Some(inventory))
            }
            None => Ok(None),
        }
    }
}
```

#### Step 4-2: sqlx による実装

sqlx による TransactionManager の実装です。

sqlx のトランザクション型 `Transaction<'_, Postgres>` はライフタイムパラメータを持つため有限ライフタイムです。
`Arc<Mutex>` パターンで `'static` Future 要件を満たすには `'static` なライフタイムである必要があるため、 `unsafe` な `transmute` を使ってライフタイムを変換します。

細かい説明になるので expand で書きますが、unsafe なコードですが安全な実装になっています。

<details>
<summary>なぜこの unsafe なコードは安全なのか？</summary>

一般論として unsafe なコードを書く際に、何を改変するために unsafe が必要で、どんな不変条件が満たされれば安全であるかを考える必要がある。

今回でいうとライフタイムを改変するために unsafe が必要であり、ライフタイムが狂って UAF が置きないことを保証する必要がある。
あまりにも当然ですが、保証するための労力を最小にするため、unsafe の影響範囲は最小限に封じ込めるべきです。
そして、unsafe による改変は想定とは異なる結果を生み出す可能性があるため、実行時に前提条件をチェックするべきです。

このあたりの考えは一般的な防御的プログラミングと Design by Contract をベースに考えればよいです。

では、今回の unsafe の安全性について考えてみましょう。
まず unsafe を使う目的ですが sqlx のトランザクションのライフタイムを有限ライフタイムから `'static` ライフタイムへと変換するものです。
`tx` は `async move` ブロックの中で定義され、 `async move` が有効な間存在できます。
実際には commit/rollback によって `tx` 消費されることで無効になります。
つまりは unsafe で `'static` ライフタイムを変更しているが、結局はブロックの中で消える運命にあるため、想定外に生存期間が長くなることはありえません。

次にトランザクションを利用する際に安全に利用しているのかどうかについてです。
`tx` は DbTransaction に所有されており、`Arc<Mutex>` でラップされます。
closure f で可変借用され、その後 `transaction()` の中で commit/rollback されます。
closure f でトランザクションが終了していたり、別スレッドから参照されている場合は、安全に commit/rollback が実行できません。
そのため `Arc::try_unwrap(db_context)` を使って他に参照がないことを保証してから実行するようになっています。

さらに Future が途中で cancel された場合、commit も rollback も呼ばれずに `SqlxDbContext` が drop される可能性があります。
この場合、sqlx のトランザクション型は Drop 実装で自動的に rollback を実行します。

また、今回は `transaction()` の内部で unsafe なコードを使っていますが、I/F 上は stable なメソッドに見えています。
unsafe は内部にカプセル化されており、前述の通りに安全性を確認できているため、`transaction()` を安全に利用することができます。

したがって、今回の unsafe なコードは安全に利用することができます。
</details>

```rust
// infrastructure/repository/sqlx_impl/src/transaction_manager.rs

use std::sync::Arc;
use tokio::sync::Mutex;
use sqlx::PgPool;

pub struct SqlxTransactionManager {
    pool: PgPool,
}

impl TransactionManager for SqlxTransactionManager {
    type DbContext = SqlxDbContext;
    type Error = anyhow::Error;

    async fn transaction<T, F, Fut>(&self, f: F) -> Result<T, Self::Error>
    where
        F: FnOnce(Arc<Mutex<Self::DbContext>>) -> Fut + Send,
        Fut: Future<Output = Result<T, Self::Error>> + Send,
        T: Send,
    {
        let pool = self.pool.clone();

        async move {
            let tx = pool.begin().await?;

            // sqlx の Transaction<'_, Postgres> は 'static でないため、
            // Arc<Mutex> パターンで使うには unsafe な transmute が必要
            let tx_static = unsafe {
                std::mem::transmute::<
                    sqlx::Transaction<'_, sqlx::Postgres>,
                    sqlx::Transaction<'static, sqlx::Postgres>,
                >(tx)
            };

            let db_context = SqlxDbContext::new(tx_static);
            let db_context = Arc::new(Mutex::new(db_context));

            match f(db_context.clone()).await {
                Ok(result) => {
                    // Arc::try_unwrap で安全に抽出して commit
                    match Arc::try_unwrap(db_context) {
                        Ok(mutex) => {
                            let context = mutex.into_inner();
                            context.commit().await?;
                            Ok(result)
                        }
                        Err(_) => {
                            Err(anyhow::anyhow!("Failed to extract context for commit"))
                        }
                    }
                }
                Err(e) => {
                    // エラー時は自動 rollback
                    match Arc::try_unwrap(db_context) {
                        Ok(mutex) => {
                            let context = mutex.into_inner();
                            let _ = context.rollback().await;
                        }
                        Err(_) => {
                            // ロールバック失敗は無視（通常発生しない）
                        }
                    }
                    Err(e)
                }
            }
        }
    }
}
```

sqlx の実装でも InventoryRepository の SELECT FOR UPDATE しつつ Inventory を取得するメソッドだけ紹介します。

```rust
impl InventoryRepository for SqlxInventoryRepository {
    type DbContext = SqlxDbContext;
    type Error = anyhow::Error;

    async fn find_by_item_id_for_update(
        &self,
        db_context: &Arc<Mutex<Self::DbContext>>,
        item_id: &ItemId,
    ) -> Result<Option<Inventory>, Self::Error> {
        let mut guard = db_context.lock().await; // db_context を排他的ロック
        let txn = guard.get_transaction(); // ロック取得後、トランザクションを取得

        // トランザクションを使って Inventory を取得
        let result = sqlx::query_as::<_, InventoryModel>(
            "SELECT item_id, quantity FROM inventories WHERE item_id = $1 FOR UPDATE"
        )
        .bind(item_id.as_uuid())
        .fetch_optional(txn)
        .await?;

        // Inventory を組み立てて返す
        match result {
            Some(model) => {
                let inventory = Inventory::new(
                    ItemId::from_uuid(model.item_id),
                    model.quantity
                )?;
                Ok(Some(inventory))
            }
            None => Ok(None),
        }
    }
}
```

### Step 5: ユースケースの実装

最後に Application 層にユースケースを実装します。
ここまで作ってきた TransactionManager や Repository を使ってビジネスロジックを組み立てましょう。

が、実は最初に提示したコンパイルエラーになるコードとほぼ同じです。

差分は利用している TransactionManager と Repository の trait にあります。
重複した内容になりますが、TransactionManager が提供し Repository が利用する `db_context` を `Arc<Mutex<DbContext>>` と定義することで、安全にトランザクション管理ができるようになりました。

```rust
// use_case/src/order_management.rs

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
```

# 本番利用する際の考慮事項

## `Arc<Mutex>` のオーバーヘッドとは

`Arc<Mutex>` のオーバーヘッドは、どれくらい激しく競合が起こるかによって決まります。
競合が頻繁に発生すれば遅くなりますし、競合がなければ高速に処理がされます。

では競合がなければ本当に高速に処理できるのか確認しましょう。

`Arc<Mutex>` 自体を生成し、競合なしのロックを取るのにかかる時間を計測してみましょう。
１回のトランザクションの処理で 10 回の Repository の関数を呼び出すという設定のもと、`Arc<Mutex>` を１回生成し、そこから 10 回のロックを取得する処理を計測しました。
実行環境は Apple M4 Pro です。

```rust
use std::sync::{Arc, Mutex};
use std::time::Instant;

fn main() {
    let n = 1_000_000;

    let start = Instant::now();
    for _ in 0..n {
        let data = Arc::new(Mutex::new(0));
        for _ in 0..10 {
            let _lock = data.lock().unwrap();
        }
    }
    let duration = start.elapsed();

    println!("Total time: {:?}", duration);
    println!("Per Arc<Mutex> creation + 10 locks: {:?}", duration / n);
}

```

```
> rustc benches/arc_mutex_overhead.rs -O -o benches/arc_mutex_overhead && benches/arc_mutex_overhead
Total time: 91.133083ms
Per Arc<Mutex> creation + 10 locks: 91ns
```

`Arc<Mutex>`のオーバーヘッドは 91 ns、一方で DB クエリは 1 ms 以上かかるため、オーバーヘッドは無視できるレベルです。

## デッドロック回避戦略

`Arc<Mutex>` で競合がなければ高速に処理できることは確認できました。
では、本当にトランザクションで `Arc<Mutex>` を使ったとき、競合は起こらないのでしょうか？

一般的な RDBMS の一般的な設定においては、１つのトランザクションは１つの DB コネクションです。
したがって、コネクション上ではクエリーの実行はシリアルな実行になります。

また、SeaORM のトランザクションも sqlx のトランザクションも Clone を実装していないため、１つのトランザクションが複数のコードから利用されることはありません。
`Arc<Mutex>` を使えばそれが可能になりますが、ロックを取ってからクエリーを実行できるので、やはりクエリーはシリアルに実行されます。

Rust はマルチスレッドなランタイムがあるため、クエリーを同時に実行するコードは書けます。
以下のように同時に Foo と Bar を作成しようとすると、片方がロックを取得し、もう片方はロックの解放を待つことになります。
ロックの確保に無駄な処理を発生させる可能性はありますが、デッドロックを引き起こしません。

```rust
tokio::join!(
    foo_repository.create_foo(&db_context, foo),
    bar_repository.create_bar(&db_context, bar),
)
```

上記のコードはシリアルに書く方が極めて自然であり、この場合はクエリーは上から順番に実行されることになります。

```rust
foo_repository.create_foo(&db_context, foo).await?;
bar_repository.create_bar(&db_context, bar).await?;
```

## パフォーマンス最適化のためのクエリーの並列実行

単一トランザクション内では、前述の通りクエリはシリアルに実行されます。
もしトランザクション間のデータ一貫性が不要で、複数のクエリを並列実行したい場合は、複数のトランザクションを使うことができます。

読み取り用途でデータの一貫性の保証が不要であり、IO 待ちを削減したい場合に有効なパターンです。

```rust
let (foo_result, bar_result) = tokio::join!(
    transaction_manager.transaction(|db_context| async move {
        foo_repository.get_foo_by_id(&db_context, foo_id).await
    }),
    transaction_manager.transaction(|db_context| async move {
        bar_repository.get_bar_by_id(&db_context, bar_id).await
    }),
);
```

ただし以下の注意点があります。

- 別トランザクションで実行されるため ACID の保証がない
- コネクションプールの枯渇のリスクが生じる
- いずれかあるいは両方が失敗する可能性があるため、エラーハンドリングが複雑になる

## エラーハンドリング戦略

今回はトランザクションのロジックにフォーカスするために anyhow を使ったイージーなエラーハンドリングを行いました。

本番環境では、DB エラーやビジネスロジックエラーを識別可能にし、適切なエラーハンドリングとリトライ戦略を実装する必要があります。
以下は構造化エラーの例です。

```rust
#[derive(thiserror::Error, Debug)]
pub enum TransactionError<BRE: std::error::Error> {
    // SeaORM や sqlx のエラー
    #[error("Database error: {0}")]
    Database(#[from] sea_orm::error::DbErr),

    #[error("Business rule violation")]
    BusinessRule(BRE),

    #[error("Concurrency conflict")]
    Concurrency,
}
```

このような構造体であれば、エラー種別に応じてエラーメッセージの生成、ロギング、リトライなどを実装可能にすることができます。
詳細な実装パターンについては本記事のスコープ外とします。

## 監視とロギング

本番環境では、トランザクションのパフォーマンス監視とエラーロギングは重要です。
OpenTelemetry などでトレーシングを取得し、SeaORM や sqlx、あるいは RDBMS 側でスロークエリーログを有効化してください。

# まとめ

本記事では、Rust におけるトランザクション管理の課題と、`Arc<Mutex>` パターンによる解決策を示しました。
もしトランザクション管理の実装で悩んでいる方がいたら、ぜひ参考にしてください。

## 本記事で実装したこと

1. `Arc<Mutex<DbContext>>` によるライフタイム問題の解決
2. 複数 Repository で安全にトランザクションを共有
3. 自動 commit/rollback の実現
4. SeaORM と sqlx の両対応

## 次のステップ

本記事のパターンは、多くのユースケースで十分な性能と安全性を提供します。
しかし実際のプロダクション環境で使用する際は、以下の拡張を検討してください。

- トランザクション分離レベルの設定: `IsolationLevel`を指定可能にする
- タイムアウト設定: 長時間トランザクションの制御
- リトライロジック: デッドロック等への対応
- 構造化エラー型: エラー種別に応じた適切なハンドリング
- 監視とロギング: OpenTelemetry統合、スロークエリ検出
- Primary/Read Replica の使い分け（読み取り専用クエリの最適化）

# 参考資料

- [Design the infrastructure persistence layer](https://learn.microsoft.com/en-us/dotnet/architecture/microservices/microservice-ddd-cqrs-patterns/infrastructure-persistence-layer-design)
- [Rust における Unit of Work の実装例](https://zenn.dev/poi2/articles/8162610d20798a)
- [SeaQL/sea-orm](https://github.com/SeaQL/sea-orm)
- [launchbadge/sqlx](https://github.com/launchbadge/sqlx)
- [Arc in std::sync](https://doc.rust-lang.org/std/sync/struct.Arc.html)
- [Mutex in std::sync](https://doc.rust-lang.org/std/sync/struct.Mutex.html)
- [The Rust Programming Language - Fearless Concurrency](https://doc.rust-lang.org/book/ch16-00-fearless-concurrency.html)
