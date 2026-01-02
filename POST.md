Rust の DB トランザクション管理方法の整理
-----

https://docs.google.com/document/d/19DicvLvGAvO8GM9Z0i-aVPz6qey8D4P-cPadOXyN3Yw/edit?tab=t.0
https://threedots.tech/post/database-transactions-in-go/
https://learn.microsoft.com/en-us/dotnet/architecture/microservices/microservice-ddd-cqrs-patterns/infrastructure-persistence-layer-design
https://martinfowler.com/eaaCatalog/unitOfWork.html

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

## 言語固有の制約

- 上記の要求をすべて解消しつつ、型パズルと所有権を満たす安全なコードを書くこと（Rust からの要求）

## SeaORM 固有の制約

Clean Architecture では抽象化のために trait を経由でトランザクションを渡す必要があります。
trait の抽象化されたトランザクションを複数の Repository で使えるようにしたいですが、具体的にどういう抽象化を行えばよいでしょうか？

トランザクションを借用で渡せればよいですが、可変借用のため複数の Repository で共有することができません。
では所有権を渡すことで解決したいですが、SeaORM のトランザクションは Clone ができないため、所有権を渡すこともできません。
そのため `Arc<Mutex<T>>` パターンを使った排他的共有を行う必要があります。

## sqlx 固有の制約

sqlx でも SeaORM と同じ抽象化を行う必要があります。
さらに SeaORM のトランザクションと同じく Clone ができないため、sqlx においても `Arc<Mutex<T>>` パターンを使った排他的共有を行う必要があります。

SeaORM はそれでよいのですが、sqlx のトランザクション `Transaction<'c, DB>` は借用ライフタイムを持つため、またもう一歩複雑になります。

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

TODO: sqlx の実装で以下を解説する。

ではなぜ `Arc` で `'static` が必要なのでしょうか？

```rust
// これがコンパイルエラーになる理由
async fn broken_example() {
    let pool = /* コネクションプール */;
    let tx = pool.begin().await?;  // tx: Transaction<'pool, Postgres>

    // エラー: 'pool は 'static より短いライフタイム
    let shared_tx: Arc<Mutex<Transaction<'static, Postgres>>> =
        Arc::new(Mutex::new(tx)); // ← ここでコンパイルエラー
}
```

`Arc` は複数のスレッド間で共有可能であり、非同期ランタイムではタスクが異なるスレッドで実行される可能性があるため、内部のデータが参照する元のリソースよりも長生きすることを防ぐ必要があります。
そのため、`Arc` に格納するデータは `'static` ライフタイム（静的ライフタイム）を持つ必要があります。
（今回のユースケースでいうと、非同期ランタイムで安全にトランザクションを共有するために静的ライフタイムが必要になっています）

そこでその矛盾を解消するために有限ライフタイムから静的ライフタイムへの変換が必要です。
それを行っているのが `unsafe` と `std::mem::transmute` を使っている部分です。

```rust
let tx_static = unsafe {
    std::mem::transmute::<
        Transaction<'_, Postgres>,
        Transaction<'static, Postgres>
    >(tx)
};
```

これによって以下の問題を解決します。

- トランザクションの所有権を明確に管理
- トランザクションの仕様を関数のスコープ内に限定

ただし、新たな問題として unsafe によってメモリー管理を手動で行うため、メモリー管理の安全性も手動で保証する必要があります。
今回の実装では unsafe は TransactionManager の `fn transaction()` の関数スコープ内に完結していてるため、関数を抜けるタイミングで unsafe のメモリー管理が終了することが保証されています。
そのため、この関数の内側では unsafe ですが、関数の外側では Rust のコンパイラーによる safe なコードが保証されます。

TODO: sqlx の実装で具体的に ①、②、③ のように番号を振って解説する

- `Arc::try_unwrap` により closure に共有したトランザクションの所有権を安全に回収している
    - このタイミングで `Arc` によるメモリーの排他的共有は終了する
    - その後、トランザクションの commit/rollback を実行して、トランザクションのセッションを終了する

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

use crate::db_context::DbContext;
use crate::inventory::aggregate::Inventory;
use crate::item::aggregate::ItemId;

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

**Rustでトランザクション管理を実現する上での最重要ポイント**: `Arc<Mutex<Self::DbContext>>` によって、型安全性を保ちながらトランザクションを複数の Repository で共有できます。

```rust
// domain/src/transaction_manager.rs

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

## Infrastructure layer: sqlx の Owned 実装

### sqlx の課題と解決策

sqlx では `Transaction<'c, DB>` が借用ライフタイムを持つため、`Arc<Mutex<T>>` パターンとの組み合わせで unsafe コードが必要になります。この問題を**所有権ベース設計**で解決します。

所有権ベースの設計により、構造的にunsafeコードの問題を解決した実装です：

```rust
// infrastructure/src/repository/sqlx_impl/transaction_manager.rs

/// Owned DbContext that owns its transaction
/// This eliminates most lifetime issues by taking ownership
pub struct OwnedSqlxDbContext {
    tx: sqlx::Transaction<'static, sqlx::Postgres>,
}

impl OwnedSqlxDbContext {
    /// Consume self and commit the transaction
    pub async fn into_commit(self) -> Result<(), sqlx::Error> {
        self.tx.commit().await
    }

    /// Consume self and rollback the transaction
    pub async fn into_rollback(self) -> Result<(), sqlx::Error> {
        self.tx.rollback().await
    }
}

impl TransactionManager for SqlxTransactionManager {
    type DbContext = OwnedSqlxDbContext;
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

            // ✅ 最小限のunsafe: 初期化時のみ
            let tx_static = unsafe {
                std::mem::transmute::<
                    sqlx::Transaction<'_, sqlx::Postgres>,
                    sqlx::Transaction<'static, sqlx::Postgres>
                >(tx)
            };

            let db_context = OwnedSqlxDbContext::new(tx_static);
            let db_context = Arc::new(Mutex::new(db_context));

            match f(db_context.clone()).await {
                Ok(result) => {
                    // ✅ 所有権取り戻し: Arc::try_unwrapで安全に抽出
                    match Arc::try_unwrap(db_context) {
                        Ok(mutex) => {
                            let owned_context = mutex.into_inner();
                            owned_context.into_commit().await?;
                            Ok(result)
                        }
                        Err(_) => {
                            Err(anyhow::anyhow!("Failed to extract owned context"))
                        }
                    }
                }
                Err(e) => {
                    match Arc::try_unwrap(db_context) {
                        Ok(mutex) => {
                            let owned_context = mutex.into_inner();
                            let _ = owned_context.into_rollback().await;
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

**改善点**:
- 🛡️ **構造的安全性**: 所有権ベース設計でunsafe範囲を最小化
- 🔄 **消費型操作**: `into_commit()` / `into_rollback()`でコンパイル時安全性
- 🎯 **明確な境界**: unsafeコードが初期化時のみに限定
- 🚀 **将来拡張性**: 完全なunsafe排除への道筋## SeaORM 実装: 参考実装

SeaORM実装は比較的シンプルで、unsafeコードは不要です：

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
        let db_context = Arc::new(Mutex::new(db_context));

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

## 実装のポイントと学び

### 1. Arc<Mutex<T>>パターンの威力

Rustでは、トランザクションのような共有リソースを複数箇所で同時に使用すると、コンパイル時に所有権エラーが発生します。
Arc（参照カウンタ）と Mutex（排他制御）を組み合わせることで、**型安全性を保ちながらトランザクションを複数の Repository で共有**できます。

### 2. unsafeコードとの向き合い方

sqlxの設計上、完全なunsafe排除は困難ですが、**所有権ベース設計**により構造的に問題を最小化できます：

- **最小限のunsafe**: 初期化時のライフタイム変換のみ
- **構造的安全性**: Arc::try_unwrapによる所有権取り戻し
- **将来への道筋**: 完全なunsafe排除の可能性を残す設計

### 3. Clean Architectureとの相性

この実装パターンは、ORM によらず汎用的に適用でき、Clean Architectureの依存性逆転の原則を満たします。
`domain` → `use_case` → `infrastructure` → `application` の依存関係が適切に保たれています。

### 4. 型システムの活用

Rustの強力な型システムを活用することで、以下を実現：

- コンパイル時のトランザクション安全性チェック
- ゼロコスト抽象化
- ORM実装の交換可能性

## まとめ

### 達成したこと

1. **型安全**: Rustの所有権システムと調和したトランザクション管理
2. **実装交換性**: SeaORM ↔ sqlx の切り替えが可能
3. **構造的安全性**: 所有権ベース設計によるunsafe問題の最小化
4. **包括的テスト**: 35個のテストによる信頼性保証
5. **Clean Architecture**: 適切な依存関係の維持

### 技術的成果

- **Arc<Mutex<DbContext>>** による安全な共有リソース管理
- **所有権ベース設計** による構造的安全性向上
- **消費型操作** による明確なリソース管理
- **Schema分離** による複数ORM並行テスト

### 今後の展望

- **完全unsafe排除**: 所有権ベース設計のさらなる発展
- **パフォーマンス最適化**: ベンチマークと最適化
- **他ORM対応**: diesel、rbatisなどへの拡張
- **分散トランザクション**: マイクロサービス対応

# reference

- [Design the infrastructure persistence layer](https://learn.microsoft.com/en-us/dotnet/architecture/microservices/microservice-ddd-cqrs-patterns/infrastructure-persistence-layer-design)
- [Unit of Work](https://martinfowler.com/eaaCatalog/unitOfWork.html)
- [【Rust】アプリケーションのDBトランザクション管理の方法を考える](https://zenn.dev/penysho/articles/a48ca73b757656)
- TODO: 自分の記事
- [Arc in std::sync](https://doc.rust-lang.org/std/sync/struct.Arc.html)
- [Mutex in std::sync](https://doc.rust-lang.org/std/sync/struct.Mutex.html)


---

Rust でのトランザクション管理は確かに複雑ですが、適切なパターンと所有権ベース設計により、**安全で実用的な**実装が可能です。`Arc<Mutex<DbContext>>`パターンは、トランザクション以外のRustマルチスレッドプログラミングにも応用できる重要な設計パターンです。

**Repository**: [rust-transaction-manager-proof-of-concept](https://github.com/poi2/rust-transaction-manager-proof-of-concept)