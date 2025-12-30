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
- それでいてトランザクションのセッションをクエリーを発行するたびに使い回せること（＝所有権に違反しないこと）
- リソース効率を最適化するため、クエリーは非同期ランタイム上で実行すること（＝非同期の型パズルを解くこと）
- Clean Architecture の依存性逆転の原則を遵守する抽象と実装を隔離を実現すること（＝抽象と実装の型パズルを解くこと）

ひとことでいうと、とても難しいということです。

# 要件の整理

ではトランザクションの実装に求められる要件はどのようなものでしょうか？
この記事は Rust の実装に持っていきたいので Rust は当然入りますが、汎用的なアプリケーションで利用可能を目指したいので、Rust x DDD x Clean Architecture x エンタープライズアプリケーションという条件下で考えましょう。
以下のような要件を置きます。

- 集約ごとに ACID トランザクションで保存できること（DDD からの要求）
- 複数の集約を同一トランザクションで保存できること（エンタープライズアプリケーションでよくある要求）
- 複数の集約を異なるトランザクションで保存できること（重たい処理を求められるアプリケーションでよくある要求）
- I/F は Domain layer に定義され、実装は Infrastructure layer に記述されること（Clean Architecture からの要求）
- パフォーマンスを重視しクエリーは非同期ランタイム上で実行できること（アプリケーションの一般的な要求）
- 上記の要求をすべて解消しつつ、型パズルと所有権を満たす安全なコードを書くこと（Rust からの要求）

# 実装

すべての要求を満たすシンプルな実装を探す旅はとても長いので、自分が見出したやりかたを共有します。

具体例があると説明書しやすいので、EC サイトで「注文の確定と、その商品の在庫を減らす」といユースケースを例に取りましょう。

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

    let order = self.transaction_manager
        .transaction(|db_context| async move { // トランザクションを開始
            // 在庫を取得
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

            Ok(order) // コミット
        })
        .await?;

    Ok(order)
}
```

transaction_manager は DB コネクションを持つ構造体で、transaction() ではまず最初にトランザクションの開始を開始します。
トランザクション内で実行したい処理は closure で外から注入できるようになっており、Application layer でビジネスロジックを定義して実行させます。
closure には db_context 経由でトランザクションが渡されており、各リポジトリーは db_context から渡されるトランザクションを利用してクエリーを実行します。
closure が成功すれば transaction() はコミットを実行し、失敗であればロールバックを実行します。

これにより、複数の異なる集約を同一トランザクションで保存することができます。

## Domain layer の実装

### TransactionManager trait

まずは TransactionManager trait から説明しましょう。

Application layer で説明と重複しますが、transaction() で closure を引数で受け取ります。
closure にはトランザクション内で実行したいビジネスロジックが定義されています。

関連型の DbContext は抽象化したトランザクションを内部に保持する想定です。
詳細は後述の説明を参照してください。

DbContext を前述の closure に `Arc<Mutex<Self::DbContext>>` でラップして渡します。
ここが重要なポイントではあるのですが、難しいポイントなので読み飛ばしていただいても構いませんが、詳細に説明すると以下のような話しです。

考えれば当たり前ではあるのですが、トランザクションは同時に実行されると実行時エラーが発生する可能性があります。
例えばですが、セッションが終了しているトランザクションにクエリーは実行しようとしてもエラーになります。
Rust の場合、コンパイル時に安全性が担保されていることを要求されるため、実行時エラーが発生するようなコードはコンパイルエラーになります。

それを回避するために、Arc, Mutex でラップすることで、相互排他的にしかトランザクションを利用できないことを型レベルで縛っています。

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

### DbContext trait

続いて DbContext です。

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

### OrderRepository trait

// TODO: ここ書き忘れている。

## Infrastructure layer の実装

2025 年においては Rust の ORM は SeaORM と SQLx がでスタンダードな選択肢となっています。
どちらでも実装可能であることを示しましょう。

### SeaORM による実装

### TransactionManager の実装

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

### DbContext の実装

SeaOrmDbContext は内部にトランザクションを持つ構造体で、DbContext を実装しています。
生成時にはトランザクションは必ずあるのですが、コミットやロールバックを実行するとトランザクションのセッションは失われてしまうため、トランザクションはオプショナルな状態で保持せざるを得ない状況にあります。

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

### Repository の実装

Repository の実装です。
db_context からトランザクションを取り出して、SeaORM 経由で INSERT を実行します。

```rust
use futures::future::BoxFuture;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, Set};
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

use domain::{db_context::DbContext, order_aggregate::Order, order_repository::OrderRepository};

use crate::{db_context::SeaOrmDbContext, order_entity::ActiveModel};

#[derive(Clone)]
pub struct SeaOrmOrderRepository;

impl OrderRepository for SeaOrmOrderRepository {
    type DbContext = SeaOrmDbContext;
    type Error = anyhow::Error;

    fn create(
        &self,
        db_context: &Arc<Mutex<Self::DbContext>>,
        order: Order,
    ) -> BoxFuture<'_, Result<Order, Self::Error>> {
        let mut guard = db_context.lock().await;
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

// TODO: ここまで書いた。

# どのような単位で一貫した状態を維持する必要があるのか？

データを一貫した状態を維持することは必須な機能ですが、どのデータの単位で一貫した状態を維持するかはアプリケーションやユースケースごとに異なります。
よくあるパターンは以下となります。

- [集約単体パターン] 集約単体で保存する
- [複数の異なる集約一括パターン] 複数の異なる集約を一括で保存する

集約単体パターンだけサポートすればよい場合は、集約に対応する Repository 単位でトランザクション管理を維持すれば良いです。
シンプルなアプリケーションであれば、集約単位で保存し、その単位でトランザクションを実行するだけで十分です。

しかし、アプリケーションの成長と共に複数の異なる集約を一括で保存する必要が生じるかもしれません。
そうでなくても、エンタープライズアプリケーションにおいては複数の異なる集約を一括で保存が必要となることが多々あります。
そのような場合は複数の異なる集約一括パターンをサポートする必要があります。

# 前提

本記事ではデータの活用（Read/Write）を Repository パターンにおける実装を行います。
依存性逆転の原則を重んじ、Repository の抽象と具象を分離し、アプリケーションにおいては抽象に依存する方針を取ります。

アプリケーションやユースケースにおいては、具象に直接依存することが許容されるケースがあります。
その場合、本記事で提案する実装は過剰な複雑性を持ち込むことにつながるリスクをはらみます。

# 集約単体パターンの実装

## 集約単体パターンの擬似コードによる説明

集約単体での保存をサポートすればよい場合、Repository のメソッド単位でトランザクションを用意すればよいです。
具体的には以下のような使い方です。

```rust
// Database Client を内部で持つ todo_repository を生成する。
let todo_repository = TodoRepositoryImpl::new(database_client);

// TodoRepository の create メソッドの内部で transaction を展開する。
todo_repository.create(todo);
```

Repository の抽象は以下です。

```rust
use async_trait::async_trait;

#[async_trait]
pub trait TodoRepository: Send + Sync {
    async fn create(&self, todo: Todo) -> Result<Todo, TodoRepositoryError>;
}
```

Repository の具象は以下です。
具象と言っていますが、実際に動作するコードは後述の章を参考にしてください。

```rust
use async_trait::async_trait;

#[derive(Debug, Clone)]
pub struct TodoRepositoryImpl {
    database_client: DatabaseClient,
}

#[async_trait]
pub trait TodoRepository: Send + Sync {
    async fn create(&self, todo: Todo) -> Result<Todo, TodoRepositoryError> {
        // 内部の database_client からトランザクションを展開する。
        let todo = self.database_client.transaction({
            // 実際には ORM のコードを利用して具体的な SQL 操作を実行する。
        }).await?

        Ok(todo)
    }
}
```

## 集約単体パターンの SeaORM による実装

TODO

## 集約単体パターンの SQLx による実装

TODO

## 集約単体パターンのまとめ

- pros
    - 実装が素直であり実装が容易である（複雑な型パズルや所有権の問題が発生しない）
- cons
    - 複数の異なる集約を一括で保存することが必要になると、大きなリファクタリングが必要となる

# 複数の異なる集約一括パターンの実装

## 複数の異なる集約一括パターンの擬似コードによる説明

## 複数の異なる集約一括パターンの SeaORM による実装

## 複数の異なる集約一括パターンの SQLx による実装

## 複数の異なる集約一括パターンのまとめ

- pros
    - 実装はとても素直だが、複雑な型パズルや所有権の問題を解消する必要がある
- cons



