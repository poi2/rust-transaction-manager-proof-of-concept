Rust における Repository を跨いだトランザクション管理の実装
-----

# はじめに

アプリケーション開発において、複数の集約を整合性も維持しつつ保存することはビジネスロジックの根幹となる課題です。
根幹でありながら、アプリケーションを取り巻く複雑な要因により、整合性の管理が複雑化してしまうことがあります。

例えば Clean Architecture ではビジネスロジック（Domain layer, Application layer）とデータの永続化の詳細（Infrastructure layer）を隔離することを要求します。
これにより複雑な集約の整合性の管理においても Domain layer, Application layer, Infrastructure layer がそれぞれの関心事のみに集中して扱いながら、全体で整合性が取れるように制御できるよう実装する必要があります。

当たり前のことですが、SQL の実行には時間がかかるので非同期で実行を行いたいですが、複数のコードが同一トランザクションを共有して SQL を実行すると、一方がトランザクションを終了したあとにもう一方がクエリーを実行した場合、ランタイムエラーを発生させるリスクがあります。

本記事では Rust のアプリケーションにおいて Clean Architecture のような抽象化を行った上で Rust の ORM を用い、使い勝手のよいトランザクション管理の方法を提案します。

# 課題の整理

## 一般的なトランザクションの実現方法

このようにトランザクションの実装やその効果的な利用は難易度が高いのですが、多くの開発者はこれまで困ることなくトランザクション機能を利用してきたことでしょう。
言語ごとのデファクトスタンダードとなる ORM を使えば、難しいことを考えることなく安全にトランザクションを利用できます。

具体例を上げると Java の AOP による宣言的トランザクションだったり、Ruby on Rails のスレッドを利用した暗黙的なオブジェクトの共有によるトランザクションは、裏側で行われている複雑な処理を綺麗に隠蔽した抽象度の高い機能を提供しています。

## 一般的なトランザクションの実現方法が Rust において難しい理由

こうした便利な仕組みは、言語の特性を活かした実装になっています。しかし Rust の場合はどうかというと、トランザクションの実装と活用というアプリケーションの根幹となる機能の実装の難易度がとても高いです。
なぜかというと Rust の強力な型システムと所有権による厳格なメモリー管理において暗黙的なオブジェクトの共有は許されておらず、一般的なアプローチを素朴に実装することができません。
前述の通り、トランザクションのセッションは複数のコードから利用されるとライフタイムエラーを発生させるリスクがあり、Rust はそれをコンパイルエラーとして扱ってしまうからです。

つまり Rust でトランザクションの実装と活用を行うためには以下のトレードオフのすべてを満たす鞍点を見つける必要があります。

1. トランザクションのセッションの所有者はただひとりである
2. それでいてトランザクションのセッションをクエリーを発行するたびに使い回せること（＝所有権に違反しないこと）
3. リソース効率を最適化するため、クエリは非同期ランタイム上で実行すること（＝非同期の型パズルを解くこと）
4. Clean Architecture の依存性逆転の原則を遵守する抽象と実装を隔離を実現すること（＝抽象と実装の型パズルを解くこと）
5. ORM 側の具体的な型の特性と上記の抽象とを合わせること（＝抽象と具体的なライブラリーの型パズルを解くこと）

ひとことでいうと、とても難しいということです。

# 要件の整理

ではトランザクションの実装に求められる要件はどのようなものでしょうか？
この記事は Rust の実装に持っていきたいので Rust は当然入りますが、汎用的なアプリケーションで利用可能を目指したいので、Rust x DDD x Clean Architecture のアプリケーションという条件下で考えましょう。
以下のような要件を置きます。

## 機能要件（トランザクションの保存パターン）

- 集約ごとに ACID トランザクションで保存できること（DDD からの要求）
- 複数の集約を同一トランザクションで保存できること（強い整合性が必要な場合）
- 複数の集約を異なるトランザクションで保存できること（パフォーマンス重視や結果整合性で十分な場合）

## アーキテクチャー要件（Clean Architecture 準拠）

- I/F は Domain layer に定義され、実装は Infrastructure layer に記述されること（Clean Architecture からの要求）

## 非機能要件（パフォーマンス）

- パフォーマンスを重視しクエリーは非同期ランタイム上で実行できること（アプリケーションの一般的な要求）

## 言語固有の制約

- 上記の要求をすべて解消しつつ、型パズルと所有権を満たす安全なコードを書くこと（Rust からの要求）

## SeaORM 固有の制約

Clean Architecture では抽象化のために trait を経由でトランザクションを渡す必要があります。
trait の抽象化されたトランザクションを複数の Repository で使えるようにしたいですが、具体的にどういう抽象化を行えばよいでしょうか？

トランザクションを借用で渡せればよいですが、可変借用のため複数の Repository で共有することができません。
では所有権を渡すことで解決したいですが、SeaORM のトランザクションは Clone ができないため、所有権を渡すこともできません。

そして当たり前ではありますが、トランザクションのセッション内においてはクエリーはシーケンシャル（直列）にしか実行できません。

それらの制約を満たすために `Arc<Mutex<T>>` パターンを使った排他的共有を行う必要があります。
（`Arc` を使うことでマルチスレッド間でデータを安全に共有を実現し、`Mutex` を使うことで排他的にデータにアクセスできるようになります）

## sqlx 固有の制約

sqlx でも SeaORM と同じ抽象化を行う必要があります。
さらに SeaORM のトランザクションと同じく Clone ができないため、sqlx においても `Arc<Mutex<T>>` パターンを使った排他的共有を行う必要があります。

SeaORM はそれでよいのですが、sqlx のトランザクション `Transaction<'c, DB>` は借用ライフタイムを持つため、さらに複雑になります。

まず `Arc<Mutex<T>>` の型制約と sqlx の `Transaction<'c, DB>` の型制約が根本的に矛盾しています。
整理すると以下です。

1. `Arc<Mutex<T>>` の要求
   - 複数のスレッド間で共有されるため、T は `Send + 'static` を要求する
   - 特に `'static` 制約により、T に有限ライフタイムの参照を含めることができない
2. sqlx のトランザクション型 `Transaction<'c, DB>` の特徴
   - `'c` は有限ライフタイム（通常はコネクションプールからの借用期間）
   - この `'c` は `'static` ではないため、`Arc<Mutex<T>>` に直接格納できない

sqlx の実装においてはこれらの複数な要件と型パズルを解くことになります。
具体的なコードは後述の sqlx の実装で解説します。

# 実装

具体例があると説明しやすいので、EC サイトで「注文の確定と、その商品の在庫を減らす」というユースケースを例に取りましょう。

## プロジェクト構造

Clean Architecture に準拠した以下のディレクトリ構造で実装を行います：

```
crates/
├── domain/                    # ビジネスロジック（共通）
│   └── src/
│       ├── inventory/
│       │   ├── aggregate.rs  # Inventory ビジネスロジック
│       │   └── repository.rs # InventoryRepository trait
│       ├── order/
│       │   ├── aggregate.rs  # Order ビジネスロジック
│       │   └── repository.rs # OrderRepository trait
│       ├── item/
│       │   └── aggregate.rs  # Item ビジネスロジック
│       ├── db_context.rs     # DbContext trait（DB抽象化）
│       └── transaction_manager.rs # TransactionManager trait
├── use_case/                  # アプリケーション層（共通）
│   └── src/
│       └── order_management.rs
├── infrastructure/            # 技術詳細
│   └── repository/
│       ├── sea_orm_impl/      # SeaORM による Repository の実装
│       │   └── src/
│       │       ├── db_context.rs
│       │       ├── transaction_manager.rs
│       │       ├── inventory_repository.rs
│       │       └── order_repository.rs
│       └── sqlx_impl/         # sqlx による Repository の実装
│           └── src/
│               ├── db_context.rs
│               ├── transaction_manager.rs
│               ├── inventory_repository.rs
│               └── order_repository.rs
├── application/              # DI + 実行可能ファイル
│   └── src/
│       ├── application_container.rs
│       ├── bin/
│       │   ├── sea_orm_app.rs
│       │   └── sqlx_app.rs
│       └── lib.rs
└── compose.yaml              # PostgreSQL Docker設定
```

この構造に要件で提示した機能・非機能要件を実現しています：

- 共通化: `domain` と `use_case` は共通のコードを利用する
- 実装交換: `infrastructure` で SeaORM と sqlx のそれぞれの Repository の実装し、実装が交換可能であることを示す
- 依存性逆転: `domain` が `infrastructure` に依存しない

## Domain layer の実装

### DbContext trait: トランザクション抽象化

まずは最も基本的な DbContext trait から説明しましょう。

DbContext trait はトランザクションを保持する前提で、トランザクションの利用とコミット、ロールバックを行う機能を抽象化したものです。
関連型の Tx は DB の種類、ORM の種類に依存した型を実装時に指定することを強制しています。

```rust
// domain/src/db_context.rs

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

### Repository trait: CRUD 抽象化

実際のユースケースに必要な２つの Repository trait を定義します。
在庫操作用の InventoryRepository と注文操作用の OrderRepository です。

この Repository は各メソッドの第一引数の `db_context: &Arc<Mutex<Self::DbContext>>` により `DbContext` を受け取ります。
`DbContext` は内部にトランザクションを持っているので、実行時に `DbContext` からトランザクションを取得します。
Repository を跨いでトランザクションを共有利用するため、`Arc<Mutex<T>>` を使い排他的な共有を実現しています。

```rust
// domain/src/inventory/repository.rs
use std::{future::Future, sync::Arc};
use tokio::sync::Mutex;

use crate::{
    db_context::DbContext,
    inventory::aggregate::Inventory,
    item::aggregate::ItemId,
};

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

    fn find_by_item_id(
        &self,
        db_context: &Arc<Mutex<Self::DbContext>>,
        item_id: &ItemId,
    ) -> impl Future<Output = Result<Option<Inventory>, Self::Error>> + Send
    where
        Self: Send;

    fn create(
        &self,
        db_context: &Arc<Mutex<Self::DbContext>>,
        inventory: Inventory,
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

### TransactionManager trait: トランザクション管理の中核

TransactionManager trait は、Application layer で使用したトランザクション管理の抽象 I/F です。
`fn transaction()` は closure を受け取ります。
closure の中身はトランザクションのブロックの中で実行する関数になっており、closure は TransactionManager が管理しているトランザクションを `Arc<Mutex<Self::DbContext>>` という型で受け取ります。
`Arc<Mutex<Self::DbContext>>` によって、型安全性を保ちながらトランザクションを複数の Repository で共有できるようになっています。

```rust
// domain/src/transaction_manager.rs

use std::{future::Future, sync::Arc};

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

## Application layer の実装

Clean Architecture でいう Application layer において以下のビジネスロジックをトランザクション処理します。

1. 在庫を取得（排他ロック）
2. 注文分の在庫を減らす（ドメインロジック）
3. 在庫を更新
4. 注文を保存

```rust
// use_case/src/order_management.rs

pub async fn create_order(&self, command: CreateOrderCommand) -> Result<Order, TM::Error> {
    let order = Order::from(command)?;

    let created_order = self
        .transaction_manager
        .transaction(|db_context| {
            let inventory_repo = Arc::clone(&self.inventory_repository);
            let order_repo = Arc::clone(&self.order_repository);
            let order = order.clone();

            async move {
                // 在庫を取得（排他ロックで同時更新を防止）
                let mut inventory = inventory_repo
                    .find_by_item_id_for_update(&db_context, order.item_id())
                    .await?
                    .ok_or_else(|| anyhow::anyhow!("Inventory not found"))?;

                // 注文分の在庫を減らす（ドメインロジック）
                inventory.decrease_stock(order.quantity())?;

                // 在庫を更新
                inventory_repo
                    .update(&db_context, inventory)
                    .await?;

                // 注文を保存
                let created_order = order_repo
                    .create(&db_context, order)
                    .await?;

                Ok(created_order)
            }
        })
        .await?;

    Ok(created_order)
}
```

## Infrastructure layer

Infrastructure layer では SeaORM と sqlx を使った２つの実装を紹介します。

### SeaORM の実装

SeaORM 実装はとてもシンプルです。

#### SeaORMTransactionManager の実装

```rust
// infrastructure/src/repository/sea_orm_impl/transaction_manager.rs

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
        // `Arc<Mutex<T>>` を使うことでトランザクションの排他的共有を実現。
        let db_context = Arc::new(Mutex::new(db_context));

        // closure `f` にはトランザクションの内部で実行する関数であり、具体的にはクエリーやドメインロジックが定義されている。
        // `f` の第一引数に `Arc<Mutex<SeaOMRDbContext>>` を渡し、`f` はそれから Repository で使うトランザクションを取得する。
        // `Arc<Mutex<SeaOMRDbContext>>` になっていることで Clone できるようになっている。
        match f(db_context.clone()).await {
            // closure `f` が成功の場合は commit を試みる。
            Ok(result) => {
                let mut guard = db_context.lock().await;
                guard.commit().await?;
                Ok(result)
            }
            // closure `f` が成功の場合は rollback を試みる。
            Err(e) => {
                let mut guard = db_context.lock().await;
                let _ = guard.rollback().await;
                Err(e)
            }
        }
    }
}
```

#### SeaORM Repository Impl の実装

Repository Impl もとても素直なコードになっています。

各関数の第一引数の `db_context: &Arc<Mutex<Self::DbContext>>` にはトランザクションが入っているので、そこからトランザクションを取得します。
安全に排他的にアクセスするため `lock()` を使って MutexGuard を取得し、トランザクションを取得している。

その後 SeaORM が提供する ORM を使ってクエリーを組み立て、トランザクション内でクエリーを実行します。

説明のために InventoryRepository の一部コードを抜粋し説明します。
OrderRepository のコードも要領は同じです。

```rust
#[derive(Clone)]
pub struct SeaOrmInventoryRepository;

impl InventoryRepository for SeaOrmInventoryRepository {
    type DbContext = SeaOrmDbContext;
    type Error = anyhow::Error;

    async fn find_by_item_id_for_update(
        &self,
        db_context: &Arc<Mutex<Self::DbContext>>,
        item_id: &ItemId,
    ) -> Result<Option<Inventory>, Self::Error> {
        // db_context にトランザクションが入っている。
        // `Arc<Mutex<T>>` の T に排他的にアクセスするために `lock()` を使って MutexGuard を取得する。
        let mut guard = db_context.lock().await;
        // 実際のトランザクションを取得する。
        let txn = guard.get_transaction();

        // SeaORM の ORM 経由でデータを取得する。
        let result = Entity::find()
            .filter(Column::ItemId.eq(*item_id.as_uuid()))
            .lock_exclusive() // SELECT FOR UPDATE
            .one(txn)
            .await?;

        match result {
            // クエリーが成功しデータが取得できた場合は、Inventory を生成して返す。
            Some(model) => {
                let inventory = Inventory::new(ItemId::from_uuid(model.item_id), model.quantity)?;
                Ok(Some(inventory))
            }
            // クエリーは成功したがデータがなかった場合は None を返す。
            None => Ok(None),
        }
    }
}
```

### sqlx の実装

sqlx の実装は SeaORM と比較すると複雑で実装難易度が高いです。

#### SqlxTransactionManager の実装

`sqlx 固有の要件` で説明した通り、トランザクションの排他的共有のために `Arc<Mutex<T>>` を使い、sqlx のトランザクション型 `Transaction<'c, DB>` の有限ライフタイムの型パズルを解く必要があります。

`Arc` は複数のスレッド間で共有可能であり、非同期ランタイムではタスクが異なるスレッドで実行される可能性があるため、内部のデータが参照する元のリソースよりも長生きすることを防ぐ必要があります。
そのため、`Arc` に格納するデータは `'static` ライフタイム（静的ライフタイム）を持つ必要があり、有限ライフタイムを含めることはできません。
（今回のユースケースでいうと、非同期ランタイムで安全にトランザクションを共有するために静的ライフタイムが必要になっています）

そこでその矛盾を解消するために有限ライフタイムから静的ライフタイムへの変換が必要です。

それらをすべて解消するのが以下のコードです。

```rust
// infrastructure/src/repository/sqlx_impl/transaction_manager.rs

impl TransactionManager for SqlxTransactionManager {
    type DbContext = SqlxDbContext;
    type Error = anyhow::Error;

    fn transaction<T, F, Fut>(&self, f: F) -> impl Future<Output = Result<T, Self::Error>> + Send
    where
        F: FnOnce(Arc<Mutex<Self::DbContext>>) -> Fut + Send,
        Fut: Future<Output = Result<T, Self::Error>> + Send,
        T: Send,
    {
        let pool = self.pool.clone();

        async move {
            let tx = pool.begin().await?;

            // 有限ライフタイムから静的ライフタイムへの変換を unsafe を使って実現。
            // unsafe は `fn transaction()` の関数スコープの中で完結しているため、
            // 関数を抜けるタイミングで unsafe のメモリー管理を抜けることが保証される。
            let tx_static = unsafe {
                std::mem::transmute::<
                    sqlx::Transaction<'_, sqlx::Postgres>,
                    sqlx::Transaction<'static, sqlx::Postgres>
                >(tx)
            };

            let db_context = SqlxDbContext::new(tx_static);
            // `Arc<Mutex<T>>` を使うことでトランザクションの排他的共有を実現。
            // unsafe を使い sqlx のトランザクション型を静的ライフタイムへと変換したことでコンパイルエラーを回避している。
            let db_context = Arc::new(Mutex::new(db_context));

            // closure `f` にはトランザクションの内部で実行する関数であり、具体的にはクエリーやドメインロジックが定義されている。
            // `f` の第一引数に `Arc<Mutex<SqlxDbContext>>` を渡し、`f` はそれから Repository で使うトランザクションを取得する。
            // `Arc<Mutex<SqlxDbContext>>` になっていることで Clone できるようになっている。
            match f(db_context.clone()).await {
                // closure `f` が成功の場合は commit を試みる。
                Ok(result) => {
                    // Arc::try_unwrap を使って安全に所有権を回収する。
                    // このタイミングで `Arc` によるメモリーの排他的共有は終了する。
                    match Arc::try_unwrap(db_context) {
                        Ok(mutex) => {
                            let mut context = mutex.into_inner();
                            context.commit().await?;
                            Ok(result)
                        }
                        Err(_) => {
                            Err(anyhow::anyhow!("Failed to extract context"))
                        }
                    }
                }
                // closure `f` が成功の場合は rollback を試みる。
                Err(e) => {
                    // Arc::try_unwrap を使って安全に所有権を回収する。
                    // このタイミングで `Arc` によるメモリーの排他的共有は終了する。
                    match Arc::try_unwrap(db_context) {
                        Ok(mutex) => {
                            let mut context = mutex.into_inner();
                            let _ = context.rollback().await;
                        }
                        Err(_) => {
                            // エラーケースではロールバック失敗を無視
                        }
                    }
                    Err(e)
                }
            }
        }
    }
}
```

#### SqlxRepository Impl の実装

Repository Impl は SeaORM も sqlx もどちらも素直なコードになっています。

各関数の第一引数の `db_context: &Arc<Mutex<Self::DbContext>>` にはトランザクションが入っているので、そこからトランザクションを取得します。
安全に排他的にアクセスするため `lock()` を使って MutexGuard を取得し、トランザクションを取得している。

その後 SeaORM が提供する ORM を使ってクエリーを組み立て、トランザクション内でクエリーを実行します。

説明のために InventoryRepository の一部コードを抜粋し説明します。
OrderRepository のコードも要領は同じです。

```rust
impl InventoryRepository for SqlxInventoryRepository {
    type DbContext = SqlxDbContext;
    type Error = anyhow::Error;

    async fn find_by_item_id_for_update(
        &self,
        db_context: &Arc<Mutex<Self::DbContext>>,
        item_id: &ItemId,
    ) -> Result<Option<Inventory>, Self::Error> {
        // db_context にトランザクションが入っている。
        // `Arc<Mutex<T>>` の T に排他的にアクセスするために `lock()` を使って MutexGuard を取得する。
        let mut guard = db_context.lock().await;
        // 実際のトランザクションを取得する。
        let txn = guard.get_transaction();

        // sqlx のクエリービルダーを使いクエリーを作り、データを取得する。
        let result = sqlx::query(
            "SELECT item_id, quantity FROM poc_for_sqlx.inventory WHERE item_id = $1 FOR UPDATE",
        )
        .bind(item_id.as_uuid())
        .fetch_optional(&mut **txn)
        .await?;

        match result {
            // クエリーが成功しデータが取得できた場合は、Inventory を生成して返す。
            Some(row) => {
                let item_id: uuid::Uuid = row.try_get("item_id")?;
                let quantity: i32 = row.try_get("quantity")?;
                let inventory = Inventory::new(ItemId::from_uuid(item_id), quantity)?;
                Ok(Some(inventory))
            }
            // クエリーは成功したがデータがなかった場合は None を返す。
            None => Ok(None),
        }
    }
}
```
# 実装のポイントと学び

## 1. `Arc<Mutex<T>>` パターンの活用

Rust では、トランザクションのような共有リソースを複数箇所で同時に使用すると、コンパイル時に所有権エラーが発生します。
`Arc` （参照カウンタ）と `Mutex` （排他制御）を組み合わせることで型安全性を保ちながらデータをマルチスレッドから排他的に扱うことができます。

Rust で並列処理を利用する場合はこれらの構造体を付き合っていく必要があります。

## 2. unsafe コードとの向き合い方

sqlx の設計および現在の Rust の言語機能においては、完全な unsafe 排除は困難ですが、不安定なコードを局所化し、それ以外のコードを安全に保つこと重要です。
今回の実装においては `async move {...}` ブロックや関数のスコープを抜けるタイミングでメモリーが解放されることで、内部では unsafe を使って型パズルを解消しながら、外部では綺麗な I/F を提供することに成功しました。

今後の展望としては、Rust のライフタイムとその変換の機能がより現実的に解釈可能になり、unsafe を必要としないコードを採用できるようになることを期待します。

## 3. Clean Architecture との相性

この実装パターンは、ORM によらず汎用的に適用でき、Clean Architecture の依存性逆転の原則を満たします。
`domain` → `use_case` → `infrastructure` → `application` の依存関係が適切に保たれています。

## まとめ

Rust x DDD x Clean Architecture のアプリケーションで利用できる Repository の設計と実装について提案を行いました。
今回の実装によりトランザクション内で複数の Repository のクエリーを実行が実現され、現実的で拡張性の高い安全な設計と実装を手に入れることができました。
この実装に苦労していたので、私が求める実装が手に入りとても満足です。
本記事の読者の方々も実装をコピーすることで、簡単に production ready な実装を手に入れる事ができます。
ぜひご活用ください。

# reference

- [Design the infrastructure persistence layer](https://learn.microsoft.com/en-us/dotnet/architecture/microservices/microservice-ddd-cqrs-patterns/infrastructure-persistence-layer-design)
- [Rust における Unit of Work の実装例](https://zenn.dev/poi2/articles/8162610d20798a)
- [SeaQL/sea-orm](https://github.com/SeaQL/sea-orm)
- [launchbadge/sqlx](https://github.com/launchbadge/sqlx)
- [Arc in std::sync](https://doc.rust-lang.org/std/sync/struct.Arc.html)
- [Mutex in std::sync](https://doc.rust-lang.org/std/sync/struct.Mutex.html)
