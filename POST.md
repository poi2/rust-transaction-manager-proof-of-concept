Rust の DB トランザクション管理方法の整理
-----

https://docs.google.com/document/d/19DicvLvGAvO8GM9Z0i-aVPz6qey8D4P-cPadOXyN3Yw/edit?tab=t.0
https://threedots.tech/post/database-transactions-in-go/
https://learn.microsoft.com/en-us/dotnet/architecture/microservices/microservice-ddd-cqrs-patterns/infrastructure-persistence-layer-design
https://martinfowler.com/eaaCatalog/unitOfWork.html

-----

# はじめに: 実はトランザクションの実装は難しい

アプリケーション開発において、複数の集約を整合性も維持しつつ保存することはビジネスロジックの根幹となる課題です。
根幹でありながら、アプリケーションを取り巻く複雑な要因により、整合性の管理が複雑化してしまうことがあります。

例えば Clean Architecture ではビジネスロジック（Domain layer, Application layer）とデータの永続化の詳細（Infrastructure layer）を隔離することを要求します。
これにより複雑な集約の整合性の管理においても Domain layer, Application layer, Infrastructure layer がそれぞれの関心事のみに集中して扱いながら、全体で整合性が取れるように制御できるよう実装する必要があります。

当たり前のことですが、SQL の実行には時間がかかるので非同期で実行を行いたいですが、複数のコードが同一トランザクションを共有して SQL を実行すると、一方がトランザクションを終了したあとにもう一方がクエリーを実行した場合、ランタイムエラーを発生させるリスクがあります。

## 一般的なトランザクションの実現方法

このようにトランザクションの実装やその効果的な利用は難易度が高いのですが、多くの開発者はこれまで困ることなくトランザクション機能を利用してきたことでしょう。
言語ごとのデファクトスタンダードとなる library を使えば、難しいことを考えることなく安全にトランザクションを利用できます。

具体例を上げると Java の AOP による宣言的トランザクションだったり、Ruby on Rails のスレッドを利用した暗黙的なオブジェクトの共有によるトランザクションは、裏側で行われている複雑な処理を綺麗に隠蔽した抽象度の高い機能を提供しています。

## 一般的なトランザクションの実現方法が Rust において難しい理由

こうした便利な仕組みは、言語の特性を活かした実装になっています。しかし Rust の場合はどうかというと、トランザクションの実装と活用というアプリケーションの根幹となる機能の実装の難易度がとても高いです。
なぜかというと Rust の強力な型システムと所有権による厳格なメモリー管理において暗黙的なオブジェクトの共有は許されておらず、一般的なアプローチを素朴に実装することができません。
前述の通り、トランザクションのセッションは複数のコードから利用されるとライフタイムエラーを発生させるリスクがあり、Rust はそれをコンパイルエラーとして扱ってしまうからです。

つまり Rust でトランザクションの実装と活用を行うためには以下のトレードオフのすべてを満たす鞍点を見つける必要があります。

- トランザクションのセッションの所有者はひとりだけしか存在してはいけない
    - SeaORM でも sqlx でも crate が提供するトランザクション型は Clone を実装していない
- それでいてトランザクションのセッションをクエリーを発行するたびに使い回せること（＝所有権に違反しないこと）
- リソース効率を最適化するため、クエリーは非同期ランタイム上で実行すること（＝非同期の型パズルを解くこと）
- Clean Architecture の依存性逆転の原則を遵守する抽象と実装を隔離を実現すること（＝抽象と実装の型パズルを解くこと）

ひとことでいうと、とても難しいということです。

# 要件の整理

ではトランザクションの実装に求められる要件はどのようなものでしょうか？
この記事は Rust の実装に持っていきたいので Rust は当然入りますが、汎用的なアプリケーションで利用可能を目指したいので、Rust x DDD x Clean Architecture x エンタープライズアプリケーションという条件下で考えましょう。
以下のような要件を置きます。

## 機能要件（トランザクションの保存パターン）

- 集約ごとに ACID トランザクションで保存できること（DDD からの要求）
- 複数の集約を同一トランザクションで保存できること（強い整合性が必要な場合）
- 複数の集約を異なるトランザクションで保存できること（パフォーマンス重視や結果整合性で十分な場合）

## アーキテクチャー要件（Clean Architecture 準拠）

- I/F は Domain layer に定義され、実装は Infrastructure layer に記述されること（Clean Architecture からの要求）

## 非機能要件（パフォーマンス）

- パフォーマンスを重視しクエリーは非同期ランタイム上で実行できること（アプリケーションの一般的な要求）

## 言語固有の要件（Rust の制約）

- 上記の要求をすべて解消しつつ、型パズルと所有権を満たす安全なコードを書くこと（Rust からの要求）

# 実装

すべての要求を満たすシンプルな実装を探す旅はとても長いので、自分が見出したやりかたを共有します。

具体例があると説明書しやすいので、EC サイトで「注文の確定と、その商品の在庫を減らす」というユースケースを例に取りましょう。

## Application layer の実装

Clean Architecture でいう Application layer において以下のビジネスロジックをトランザクション処理します。

1. 在庫を取得
2. 注文分の在庫を減らす
3. 在庫を更新
4. 注文を保存

コードのイメージは以下のとおりです。

```rust
pub async fn create_order(&self, command: CreateOrderCommand) -> Result<Order, Box<dyn std::error::Error>> {
    let order = Order::from(command)?;

    let created_order = self.transaction_manager
        .transaction(|db_context| async move { // トランザクションを開始
            // 在庫を取得（排他ロックで同時更新を防止）
            let mut inventory = self
                .inventory_repository
                .find_by_item_id_for_update(&db_context, order.item_id())
                .await?;
            // 注文分の在庫を減らす
            inventory.decrease_stock(order.quantity())?;

            // 在庫を更新
            self.inventory_repository
                .update(&db_context, inventory)
                .await?;

            // 注文を保存
            self.order_repository
                .create(&db_context, order)
                .await?;

            Ok(order) // 処理成功時は自動コミット
        })
        .await?;

    Ok(created_order)
}
```

transaction_manager は DB コネクションを持つ構造体で、transaction() ではまず最初にトランザクションを開始します。
トランザクション内で実行したい処理は closure で外から注入でき、Application layer でビジネスロジックを定義して実行させます。
closure には db_context 経由でトランザクションが渡されており、各リポジトリーは db_context から渡されるトランザクションを利用してクエリーを実行します。
closure が成功すれば transaction() はコミットを実行し、失敗であればロールバックを実行します。

これにより、複数の異なる集約を同一トランザクションで保存することができます。

## Domain layer の実装

### DbContext trait

まずは最も基本的な DbContext trait から説明しましょう。

DbContext trait はトランザクションを保持する前提で、トランザクションの利用とコミット、ロールバックを行う機能を抽象化したものです。
関連型の Tx は DB の種類、ORM の種類に依存した型を実装時に指定することを強制しています。
get_transaction() で指定した Tx が取得できるようになっています。

```rust
// domain_crate/src/db_context.rs

/// Domain layer database context abstraction
#[allow(async_fn_in_trait)]
pub trait DbContext: Send + Sync {
    type Tx: Send;
    type Error: Send + Sync + 'static;

    /// Get mutable reference to the underlying database transaction
    fn get_transaction(&mut self) -> &mut Self::Tx;

    /// Commit transaction (usually called by TransactionManager)
    async fn commit(&mut self) -> Result<(), Self::Error>;

    /// Rollback transaction (usually called by TransactionManager)
    async fn rollback(&mut self) -> Result<(), Self::Error>;
}
```

### Repository trait

続いて Repository trait について説明します。
実際のユースケースに必要な２つの Repository trait を定義します。在庫操作用の InventoryRepository と注文操作用の OrderRepository です。

```rust
// domain_crate/src/order_repository.rs
pub trait OrderRepository: Send + Sync {
    type DbContext: DbContext;
    type Error: Send + Sync + 'static;

    fn create(
        &self,
        db_context: &Arc<Mutex<Self::DbContext>>,
        order: Order,
    ) -> impl Future<Output = Result<Order, Self::Error>> + Send
    where
        Self: Send;
}
```

```rust
// domain_crate/src/inventory_repository.rs
pub trait InventoryRepository: Send + Sync {
    type DbContext: DbContext;
    type Error: Send + Sync + 'static;

    fn find_by_item_id_for_update(
        &self,
        db_context: &Arc<Mutex<Self::DbContext>>,
        item_id: ItemId,
    ) -> impl Future<Output = Result<Inventory, Self::Error>> + Send
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

### TransactionManager trait

続いて TransactionManager trait です。

TransactionManager trait は、Application layer で使用したトランザクション管理の抽象 I/F です。

関連型の DbContext は上記で説明したトランザクション抽象化の仕組みです。

DbContext を前述の closure に `Arc<Mutex<Self::DbContext>>` でラップして渡します。
これが Rust でトランザクション管理を実現する上での重要なポイントです。

Rustでは、トランザクションのような共有リソースを複数箇所で同時に使用すると、コンパイル時に所有権エラーが発生します。
Arc（参照カウンタ）と Mutex（排他制御）を組み合わせることで、型安全性を保ちながらトランザクションを複数の Repository で共有できます。

```rust
// domain_crate/src/transaction_manager.rs

use std::future::Future;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::db_context::DbContext;

/// Transaction Manager trait
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

## Infrastructure layer の実装

2025 年においては Rust の ORM は SeaORM と sqlx がデファクトスタンダードな選択肢となっています。
どちらでも実装可能であることを示しましょう。

### SeaORM による実装

#### TransactionManager の実装

SeaOrmTransactionManager は内部に DB コネクションを保持し、TransactionManager を実装しています。

transaction() 内部では begin() でトランザクションを生成し、SeaOrmDbContext でラップします。
それを前述の `Arc<Mutex<...>>` という Rust の型で更にラップすることで、相互排他的なアクセスを強制します。

そして引数で渡された closure f に SeaOrmDbContext を渡して実行し、成功であればコミット、失敗であればロールバックを実行します。

```rust
use sea_orm::{Database, DatabaseConnection, TransactionTrait};
use std::sync::Arc;
use tokio::sync::Mutex;

use domain::db_context::DbContext;
use domain::transaction_manager::TransactionManager;

use crate::db_context::SeaOrmDbContext;

/// SeaORM TransactionManager implementation
pub struct SeaOrmTransactionManager {
    db: DatabaseConnection,
}

impl SeaOrmTransactionManager {
    pub async fn new(database_url: &str) -> Result<Self, anyhow::Error> {
        let db = Database::connect(database_url).await?;
        Ok(Self { db })
    }
}

impl TransactionManager for SeaOrmTransactionManager {
    type DbContext = SeaOrmDbContext;
    type Error = anyhow::Error;

    fn transaction<T, F, Fut>(
        &self,
        f: F,
    ) -> impl std::future::Future<Output = Result<T, Self::Error>> + Send
    where
        F: FnOnce(Arc<Mutex<Self::DbContext>>) -> Fut + Send,
        Fut: std::future::Future<Output = Result<T, Self::Error>> + Send,
        T: Send,
    {
        let db = self.db.clone();
        async move {
            let txn = db.begin().await?;
            let db_context = SeaOrmDbContext::new(txn);
            let db_context = Arc::new(Mutex::new(db_context));

            match f(db_context.clone()).await {
                Ok(result) => {
                    let mut guard = db_context.lock().await;
                    guard.commit().await?;
                    Ok(result)
                }
                Err(e) => {
                    let mut guard = db_context.lock().await;
                    guard.rollback().await?;
                    Err(e)
                }
            }
        }
    }
}
```

#### DbContext の実装

SeaOrmDbContext は内部にトランザクションを持つ構造体で、DbContext を実装しています。
生成時にはトランザクションは必ずあるのですが、コミットやロールバックを実行するとトランザクションのセッションは失われてしまいます。
実行時にトランザクションのセッションが取得できるパターンとできないパターンの両方があり得るため、トランザクションはオプショナルな状態で保持せざるを得ない状況にあります。

また、今回は説明のために anyhow による簡易なエラーハンドリングにしています。
production で利用する際は thiserror を使って具体的なエラーにマッピングし、適切にエラーハンドリングできるようにすることを強くおすすめします。

```rust
use sea_orm::DatabaseTransaction;

use domain::db_context::DbContext;

/// SeaORM DatabaseTransaction wrapper
pub struct SeaOrmDbContext {
    transaction: Option<DatabaseTransaction>,
}

impl SeaOrmDbContext {
    pub fn new(transaction: DatabaseTransaction) -> Self {
        Self {
            transaction: Some(transaction),
        }
    }
}

impl DbContext for SeaOrmDbContext {
    /// SeaORM DatabaseTransaction type
    /// This provides direct access to sea_orm::DatabaseTransaction for:
    /// - Using SeaORM's type-safe entity operations
    /// - Full SeaORM feature access (relations, active models, etc.)
    /// - Native SeaORM operations without abstraction overhead
    type Tx = DatabaseTransaction;
    type Error = anyhow::Error;

    /// Get mutable reference to the underlying SeaORM transaction
    fn get_transaction(&mut self) -> &mut Self::Tx {
        self.transaction
            .as_mut()
            .expect("Transaction already consumed")
    }

    async fn commit(&mut self) -> Result<(), Self::Error> {
        if let Some(tx) = self.transaction.take() {
            tx.commit().await?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Transaction already consumed"))
        }
    }

    async fn rollback(&mut self) -> Result<(), Self::Error> {
        if let Some(tx) = self.transaction.take() {
            tx.rollback().await?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Transaction already consumed"))
        }
    }
}
```

#### Repository の実装

Repository の実装です。
在庫操作用の InventoryRepository と注文操作用の OrderRepository です。

db_context からトランザクションを取り出して、SeaORM 経由で INSERT を実行します。

TODO: コードを書いてちゃんと確認をする。

```rust
// sea_orm_repository/src/order_repository.rs
#[derive(Clone)]
pub struct SeaOrmOrderRepository;

impl OrderRepository for SeaOrmOrderRepository {
    type DbContext = SeaOrmDbContext;
    type Error = anyhow::Error;

    async fn create(
        &self,
        db_context: &Arc<Mutex<Self::DbContext>>,
        order: Order,
    ) -> Result<Order, Self::Error> {
        let mut guard = db_context.lock().await?;
        let txn = guard.get_transaction();

        let order = ActiveModel {
            id: Set(order.id()),
            item_id: Set(order.item_id()),
            quantity: Set(order.quantity())
        };

        order.insert(txn).await?;

        Ok(order)
    }
}
```

```rust
// sea_orm_repository/src/inventory_repository.rs
#[derive(Clone)]
pub struct SeaOrmInventoryRepository;

impl InventoryRepository for SeaOrmInventoryRepository {
    type DbContext = SeaOrmDbContext;
    type Error = anyhow::Error;

    async fn find_by_item_id_for_update(
        &self,
        db_context: &Arc<Mutex<Self::DbContext>>,
        item_id: ItemId,
    ) -> Result<Inventory, Self::Error> {
        let mut guard = db_context.lock().await?;
        let txn = guard.get_transaction();

        let result = InventoryEntity::find()
            .filter(Column::Id.eq(item_id))
            .one(txn)
            .await?;

        Ok(result.map(|model| Inventory::new(result.id, result.quantity)))
    }

    async fn update(
        &self,
        db_context: &Arc<Mutex<Self::DbContext>>,
        inventory: Inventory,
    ) -> Result<Inventory, Self::Error> {
        let mut guard = db_context.lock().await?;
        let txn = guard.get_transaction();

        let active_inventory = ActiveModel {
            id: Set(inventory.id()),
            quantity: Set(inventory.quantity().i32),
        };

        active_inventory.update(txn).await?;

        Ok(inventory)
    }
}
```

### sqlx による実装

#### TransactionManager の実装

sqlxTransactionManager は内部に PostgreSQL のコネクションプールを保持し、TransactionManager を実装しています。

transaction() 内部では pool.begin() でトランザクションを生成し、SqlxDbContext でラップします。
SeaORMの実装と同様に `Arc<Mutex<...>>` でラップすることで、相互排他的なアクセスを強制します。

```rust
use std::future::Future;
use std::sync::Arc;
use tokio::sync::Mutex;

use sqlx::PgPool;
use domain::{db_context::DbContext, transaction_manager::TransactionManager};
use crate::db_context::SqlxDbContext;

/// sqlx TransactionManager implementation using PostgreSQL
pub struct SqlxTransactionManager {
    pool: PgPool,
}

impl SqlxTransactionManager {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl TransactionManager for SqlxTransactionManager {
    type DbContext = SqlxDbContext<'static>;
    type Error = anyhow::Error;

    async fn transaction<T, F, Fut>(&self, f: F) -> Result<T, Self::Error>
    where
        F: FnOnce(Arc<Mutex<Self::DbContext>>) -> Fut + Send,
        Fut: Future<Output = Result<T, Self::Error>> + Send,
        T: Send,
    {
        let pool = self.pool.clone();
        let tx = pool.begin().await?;
        let db_context = Arc::new(Mutex::new(SqlxDbContext::new(tx)));

        match f(db_context.clone()).await {
            Ok(result) => {
                let mut guard = db_context.lock().await;
                guard.commit().await?;
                Ok(result)
            }
            Err(e) => {
                let mut guard = db_context.lock().await;
                let _ = guard.rollback().await;
                Err(e)
            }
        }
    }
}
```

#### DbContext の実装

SqlxDbContext は内部に sqlx の Transaction を持つ構造体で、DbContext を実装しています。
ライフタイム管理が必要な点が SeaORM と異なりますが、基本的な構造は同じです。

```rust
use sqlx::{Postgres, Transaction};
use domain::db_context::DbContext;

/// sqlx::Transaction wrapper for PostgreSQL
pub struct SqlxDbContext<'a> {
    transaction: Option<Transaction<'a, Postgres>>,
}

impl<'a> SqlxDbContext<'a> {
    pub fn new(transaction: Transaction<'a, Postgres>) -> Self {
        Self {
            transaction: Some(transaction),
        }
    }
}

impl<'a> DbContext for SqlxDbContext<'a> {
    type Tx = Transaction<'a, Postgres>;
    type Error = anyhow::Error;

    fn get_transaction(&mut self) -> &mut Self::Tx {
        self.transaction
            .as_mut()
            .expect("Transaction already consumed")
    }

    async fn commit(&mut self) -> Result<(), Self::Error> {
        if let Some(tx) = self.transaction.take() {
            tx.commit().await?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Transaction already consumed"))
        }
    }

    async fn rollback(&mut self) -> Result<(), Self::Error> {
        if let Some(tx) = self.transaction.take() {
            tx.rollback().await?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Transaction already consumed"))
        }
    }
}
```

#### Repository の実装

Repository の実装では、sqlx の query マクロを使用して型安全な SQL を実行します。
db_context からトランザクションを取り出して、sqlx 経由で直接 SQL を実行します。

```rust
// sqlx_repository/src/order_repository.rs
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

use domain::db_context::DbContext;
use domain::{order_aggregate::Order, order_repository::OrderRepository};

use crate::db_context::SqlxDbContext;

/// OrderRepository implementation using sqlx
#[derive(Clone)]
pub struct SqlxOrderRepository;

impl OrderRepository for SqlxOrderRepository {
    type DbContext = SqlxDbContext<'static>;
    type Error = anyhow::Error;

    async fn create(
        &self,
        db_context: &Arc<Mutex<Self::DbContext>>,
        order: Order,
    ) -> Result<Order, Self::Error> {
        let mut guard = db_context.lock().await;
        let txn = guard.get_transaction();

        sqlx::query("INSERT INTO orders (id, item_id, quantity) VALUES ($1, $2, $3)")
            .bind(order.id())
            .bind(order.item_id())
            .bind(order.quantity())
            .execute(&mut **txn)
            .await?;

        Ok(order)
    }
}
```

```rust
// sqlx_repository/src/inventory_repository.rs
use std::sync::Arc;
use tokio::sync::Mutex;

use domain::db_context::DbContext;
use domain::{inventory_aggregate::Inventory, inventory_repository::InventoryRepository, item_id::ItemId};

use crate::db_context::SqlxDbContext;

/// InventoryRepository implementation using sqlx
#[derive(Clone)]
pub struct SqlxInventoryRepository;

impl InventoryRepository for SqlxInventoryRepository {
    type DbContext = SqlxDbContext<'static>;
    type Error = anyhow::Error;

    async fn find_by_item_id_for_update(
        &self,
        db_context: &Arc<Mutex<Self::DbContext>>,
        item_id: ItemId,
    ) -> Result<Inventory, Self::Error> {
        let mut guard = db_context.lock().await;
        let txn = guard.get_transaction();

        let result = sqlx::query_as::<_, (ItemId, i32)>(
            "SELECT item_id, quantity FROM inventory WHERE item_id = $1 FOR UPDATE"
        )
        .bind(item_id)
        .fetch_one(&mut **txn)
        .await?;

        Ok(Inventory::new(result.0, result.1))
    }

    async fn update(
        &self,
        db_context: &Arc<Mutex<Self::DbContext>>,
        inventory: Inventory,
    ) -> Result<Inventory, Self::Error> {
        let mut guard = db_context.lock().await;
        let txn = guard.get_transaction();

        sqlx::query("UPDATE inventory SET quantity = $1 WHERE item_id = $2")
            .bind(inventory.quantity())
            .bind(inventory.item_id())
            .execute(&mut **txn)
            .await?;

        Ok(inventory)
    }
}
```

## まとめ

Rust でのトランザクション管理が難しいという課題は SeaORM や sqlx のトランザクションが安易な Clone やライフタイムを回避できないという難しさがあることを示しました。
`Arc<Mutex<...>>` を使った相互排他的なアクセス制御によって、同一トランザクション内で複数の Repository のクエリー実行をサポートするパターンを示しました。
また、このトランザクションの管理パターンは、ORM によらず汎用的に適用できることを示しました。

さらに `Arc<Mutex<...>>` は複数の所有者が安全に共有データを参照・変更する必要がある場合に必ず必要になるパターンです。
トランザクション以外にも Rust でマルチスレッドで利用されるデータを管理する際には登場します。
トランザクションと同じように抽象化することで型パズルを乗り越えることができます。
