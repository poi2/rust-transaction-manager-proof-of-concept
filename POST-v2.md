Rust アプリケーションにおける実践的トランザクション設計
-----

# Rust におけるトランザクション管理の現実解

Rust でエンタープライズアプリケーションを構築する際、最初に直面する壁の一つがトランザクション管理です。
所有権システムの制約により、他言語では当たり前のパターンが適用できず、多くの開発者が実装に悩むポイントとなっています。

本記事では、実際のプロダクション環境で使用できる実装パターンを、具体的なコード例とともに解説します。
Rust におけるスタンダードな DB アクセスライブラリーである SeaORM と sqlx の両方での実装を通じて、実践的なアプローチを提示します。

# なぜRustでトランザクション管理は困難なのか

## 所有権システムがもたらす制約

まず、なぜ Rust でトランザクション管理が困難なのかを、具体的なコード例で見てみましょう。
以下は use case の中で複数の Repository が１つのトランザクションを使ってビジネスロジックを実行しようとするコードの例です。

トランザクション管理を TransactionManager に委譲し、commit/rollback を自動化するために Transaction Block パターンを実装しようとすると、以下のようなコードになります：

```rust
async fn create_order(&self, command: CreateOrderCommand) -> Result<Order, String> {
    let order = Order::try_from(command).unwrap();

    let created_order = self
        .transaction_manager
        .transaction(|db_context| {
            Box::pin(async move {
                // 在庫を取得（排他ロックで同時更新を防止）
                let mut inventory = self
                    .inventory_repository
                    .find_by_item_id_for_update(db_context, order.item_id())
                    .await?;

                // 在庫を更新
                self.inventory_repository
                    .update(db_context, inventory)
                    .await?;

                // 注文を作成
                let created_order = self
                    .order_repository
                    .create(db_context, order)
                    .await?;

                Ok(created_order)
            })
        })
        .await?;

    Ok(created_order)
}
```

このコードをコンパイルしようとすると、以下のエラーが発生します：

```text
error: lifetime may not live long enough
  --> crates/use_case/src/examples/pattern2_transaction_block_fails.rs:99:21
   |
93 |           async fn create_order(&self, command: CreateOrderCommand) -> Result<Order, String> {
   |                                 - let's call the lifetime of this reference `'1`
...
99 | /                     Box::pin(async move {
   | |_____________________^
   | |
   | returning this value requires that `'1` must outlive `'static`
```

このエラーは、`async move` ブロック内で `db_context: &mut DbContext` を使用しようとすることで発生します。
`async move` ブロックは `'static` な Future を要求しますが、`db_context` のライフタイムはクロージャ引数に束縛されているため、この要件を満たせません。
根本的に、この設計では `&mut` の借用を複数の Repository 呼び出しで共有できないのです。

<details>
<summary>トランザクションの可変借用が制限される背景</summary>
トランザクション構造体は、DB との通信を通じて内部状態を絶えず変化させるため、可変性を必要とします。
マルチスレッドで１つのトランザクションに対して無秩序に利用できてしまうと、

- 通信が競合して意図しない結果になるリスク（データ競合の問題）
- 誰かが commit/rollback して無効になったトランザクションを別の誰かが利用してしまうリスク（UAF（Use-After-Free）の問題）

という危険性を抱えることになります。
このような問題を回避するために Rust では可変借用を同時に持てるのは１つのスレッドだけという制約を課しています。
</details>

## 他言語なら簡単な理由

他言語でトランザクション管理が簡単な理由を理解することで、Rust での課題がより明確になります。

以下では Java、Golang、C#、Ruby においてどのようにトランザクション管理を行っているか小さな例を持って説明しています。

各言語ごとに好まれるデザインは様々ですが、それぞれスタンダードな解決方法が定まっています。
一方 Rust は、コンパイル時の厳格な所有権チェックにより、将来したアプローチを簡単に導入できない状況にあります。

### Java: Spring @Transactional

Java の Spring フレームワークでは `@Transactional` アノテーションを使うことで、トランザクション管理を行えます。

```java
@Transactional
public void createOrder(Order order) {
    inventoryRepository.update(inventory);
    orderRepository.create(order);
}
```

### Golang: context 伝播

Golang では context を経由して必要なデータを伝播させます。
context にトランザクションオブジェクトをセットし利用する方法があります。

```go
func CreateOrder(ctx context.Context, order Order) error {
    tx, err := db.BeginTx(ctx, nil)
    if err != nil {
        return err
    }
    defer tx.Rollback() // 自動ロールバック

    // context を通じてトランザクションを伝播
    if err := inventoryRepo.Update(ctx, tx, inventory); err != nil {
        return err
    }
    if err := orderRepo.Create(ctx, tx, order); err != nil {
        return err
    }

    return tx.Commit()
}
```

### C#: Entity Framework

C# では DB コンテキストが変更を自動で追跡し、Commit で一括保存する Unit of Work パターンが一般的です。

```csharp
using (var transaction = context.Database.BeginTransaction())
{
    inventoryRepository.Update(inventory);
    orderRepository.Create(order);
    transaction.Commit();
}
```

### Ruby: Ruby on Rails ActiveRecord

Ruby では Ruby on Rails の ActiveRecord によってトランザクション管理は隠蔽されており、宣言して書くだけで要件を満たすことができます。

```ruby
ActiveRecord::Base.transaction do
  inventory.update!(...)
  order.save!(...)
end
```


## Rust での技術的制約

Rust でトランザクション管理を実現しつつ現実的な実装に落とし込むには、以下の制約をすべて満たす必要があります：

1. 排他的所有権: トランザクションの所有者を常に一意にすること
2. 安全な共有: 複数 Repository でトランザクションを共有しつつ、同時アクセス防止すること
3. 非同期対応: 非同期ランタイムでの型安全性確保
4. 抽象化: Clean Architecture などの依存性逆転原則への対応
5. ORM 互換性: SeaORM、sqlx 等の具体的なライブラリとの統合

これらすべてを同時に満たす「現実解」が必要になります。

# 実践的解決パターン

## パターン1: Arc<Mutex>による共有トランザクション

### 設計概要

最も実用的で広く採用できるのが、`Arc<Mutex>`による排他的共有パターンです。

```rust
use std::sync::Arc;
use tokio::sync::Mutex;

// トランザクションを Arc<Mutex> でラップ
let db_context = Arc::new(Mutex::new(transaction_context));

// 複数のRepositoryで安全に共有
inventory_repo.update(&db_context, inventory).await?;
order_repo.create(&db_context, order).await?;
```

この設計により、コンパイル時安全性を保ちながらトランザクションの共有が可能になります。

### 適用場面

- **複数Repository**: トランザクション共有が必要
- **Clean Architecture**: 依存性逆転の原則を維持したい
- **型安全性**: コンパイル時チェックを最大限活用
- **チーム開発**: 学習コストを抑えつつ安全性を確保

## 実装ガイド: 段階的アプローチ

実装は以下の段階を追って進めることで、理解しながら安全に構築できます。

### Step 1: トランザクション抽象化の設計

まず、データベースやORMに依存しない抽象化レイヤーを定義します。

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

この抽象化により、具体的なORM実装と独立したドメイン設計が可能になります。

### Step 2: Repository traitの定義

Repository では `Arc<Mutex<DbContext>>` を受け取る設計とします。

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

### Step 3: TransactionManagerの実装

トランザクション管理の中核となるTransactionManagerを定義します。

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

### Step 4: 具体的実装例 (SeaORM)

SeaORMでの具体実装を示します。

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
                // Arc::try_unwrap で安全に抽出してcommit
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
                // エラー時は自動rollback
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

### Step 5: Repository実装

実際のRepository実装では、`Arc<Mutex>`から安全にトランザクションを取得します。

```rust
impl InventoryRepository for SeaOrmInventoryRepository {
    type DbContext = SeaOrmDbContext;
    type Error = anyhow::Error;

    async fn find_by_item_id_for_update(
        &self,
        db_context: &Arc<Mutex<Self::DbContext>>,
        item_id: &ItemId,
    ) -> Result<Option<Inventory>, Self::Error> {
        let mut guard = db_context.lock().await;
        let txn = guard.get_transaction();

        let result = Entity::find()
            .filter(Column::ItemId.eq(*item_id.as_uuid()))
            .lock_exclusive() // SELECT FOR UPDATE
            .one(txn)
            .await?;

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

### Step 6: ビジネスロジックでの使用

Application層でトランザクションを使用します。

```rust
// use_case/src/order_management.rs

pub async fn create_order(&self, command: CreateOrderCommand) -> Result<Order, TM::Error> {
    let order = Order::try_from(command)?;

    let created_order = self
        .transaction_manager
        .transaction(|db_context| {
            let inventory_repo = Arc::clone(&self.inventory_repository);
            let order_repo = Arc::clone(&self.order_repository);
            let order = order.clone();

            async move {
                // 在庫を取得（排他ロック）
                let mut inventory = inventory_repo
                    .find_by_item_id_for_update(&db_context, order.item_id())
                    .await?
                    .ok_or_else(|| anyhow::anyhow!("Inventory not found"))?;

                // ドメインロジック：在庫減少
                inventory.decrease_stock(order.quantity())?;

                // 在庫更新
                inventory_repo.update(&db_context, inventory).await?;

                // 注文作成
                let created_order = order_repo.create(&db_context, order).await?;

                Ok(created_order)
            }
        })
        .await?;

    Ok(created_order)
}
```

## パターン2: Unit of Work

より高度な場面では、Unit of Workパターンも検討できます。

### 基本概念

```rust
pub struct UnitOfWork {
    changes: Vec<Box<dyn Change>>,
    db_context: Arc<Mutex<dyn DbContext>>,
}

impl UnitOfWork {
    pub fn register_new<T>(&mut self, entity: T) {
        self.changes.push(Box::new(InsertChange::new(entity)));
    }

    pub fn register_dirty<T>(&mut self, entity: T) {
        self.changes.push(Box::new(UpdateChange::new(entity)));
    }

    pub async fn commit(&mut self) -> Result<(), Error> {
        for change in &self.changes {
            change.execute(&self.db_context).await?;
        }
        self.changes.clear();
        Ok(())
    }
}
```

### 適用場面

- **複雑なビジネスロジック**: 多数のエンティティ操作がある場合
- **バッチ処理**: まとめてコミットしたい場合
- **監査ログ**: 変更履歴の追跡が必要

### 実装コスト vs 効果

Unit of Workは強力ですが実装コストが高く、多くの場面では `Arc<Mutex>` パターンで十分です。

# 本番運用での考慮事項

## パフォーマンス特性の理解

### Arc<Mutex>のオーバーヘッド

実際のベンチマーク結果（参考値）：

```rust
// 直接アクセス vs Arc<Mutex>アクセス
// 単純なクエリ実行での比較

Direct access:     100ns per operation
Arc<Mutex> access: 150ns per operation (50% overhead)

// ただし実際のデータベースI/O (1ms+) に比べれば無視できるレベル
```

### ボトルネック分析

```rust
// パフォーマンス問題が起きやすいパターン
async fn performance_anti_pattern() {
    for item in large_item_list {
        // 各反復でMutexロック取得・解放
        let guard = db_context.lock().await;
        repository.process_item(&guard, item).await;
        // ここでロック解放
    }
}

// 改善版
async fn performance_optimized() {
    let guard = db_context.lock().await; // 一度だけロック
    for item in large_item_list {
        repository.process_item_with_guard(&guard, item).await;
    }
    // 最後にロック解放
}
```

## 運用上の注意点

### デッドロック回避戦略

```rust
// デッドロックが起きやすいパターン
async fn deadlock_prone() {
    let guard1 = db_context1.lock().await;
    let guard2 = db_context2.lock().await; // 他のタスクが逆順でロックすると危険
}

// 改善：ロック順序の統一
async fn deadlock_safe() {
    // 常に同じ順序でロック取得
    let (guard1, guard2) = if id1 < id2 {
        (db_context1.lock().await, db_context2.lock().await)
    } else {
        (db_context2.lock().await, db_context1.lock().await)
    };
}
```

### エラーハンドリング戦略

```rust
// 構造化エラーハンドリング
#[derive(thiserror::Error, Debug)]
pub enum TransactionError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Business rule violation: {message}")]
    BusinessRule { message: String },

    #[error("Concurrency conflict")]
    Concurrency,

    #[error("Transaction timeout")]
    Timeout,
}

// エラー別の回復戦略
match transaction_result {
    Err(TransactionError::Concurrency) => {
        // 再試行戦略
        retry_with_backoff().await
    }
    Err(TransactionError::BusinessRule { .. }) => {
        // ログ記録して呼び出し元にエラー返却
        log::warn!("Business rule violation: {}", err);
        return Err(err);
    }
    _ => return Err(err),
}
```

### 監視とロギング

```rust
// トランザクション実行時間の監視
#[tracing::instrument(skip(f))]
async fn transaction<T, F, Fut>(&self, f: F) -> Result<T, Self::Error>
where
    F: FnOnce(Arc<Mutex<Self::DbContext>>) -> Fut + Send,
    Fut: Future<Output = Result<T, Self::Error>> + Send,
{
    let start = std::time::Instant::now();

    let result = /* トランザクション実行 */;

    let duration = start.elapsed();
    if duration > Duration::from_millis(1000) {
        tracing::warn!(duration_ms = duration.as_millis(), "Slow transaction detected");
    }

    result
}
```

## チーム導入指針

### 段階的移行戦略

**Phase 1: 学習フェーズ (1-2週間)**
```rust
// 既存コードを部分的に移行
// 単一Repositoryから始める
let result = transaction_manager.transaction(|db_context| async move {
    repository.simple_operation(&db_context, data).await
}).await?;
```

**Phase 2: 実践フェーズ (2-4週間)**
```rust
// 複数Repository連携
// ビジネスロジックの複雑化
transaction_manager.transaction(|db_context| async move {
    let inventory = inventory_repo.find_for_update(&db_context, id).await?;
    let updated = business_logic.process(inventory)?;
    inventory_repo.update(&db_context, updated).await?;
    order_repo.create(&db_context, order).await
}).await?;
```

**Phase 3: 最適化フェーズ (継続的)**
```rust
// パフォーマンス最適化
// エラーハンドリングの充実
// 監視の導入
```

### コードレビューポイント

1. **トランザクション境界**: 適切なスコープ設定
2. **エラーハンドリング**: rollback の確実な実行
3. **パフォーマンス**: 不要な長時間ロック
4. **テスタビリティ**: モックしやすい設計

### テスト戦略

```rust
#[cfg(test)]
mod tests {
    use super::*;

    // インメモリデータベースでの統合テスト
    #[tokio::test]
    async fn test_transaction_rollback() {
        let pool = create_test_pool().await;
        let tx_manager = TestTransactionManager::new(pool);

        let result = tx_manager.transaction(|db_context| async move {
            repository.create(&db_context, valid_data).await?;
            repository.create(&db_context, invalid_data).await // エラーになる
        }).await;

        assert!(result.is_err());
        // ロールバック確認
        assert_eq!(repository.count().await, 0);
    }

    // モックを使った単体テスト
    #[tokio::test]
    async fn test_business_logic() {
        let mut mock_repo = MockInventoryRepository::new();
        mock_repo.expect_find_for_update()
                 .returning(|_| Ok(Some(test_inventory())));

        let result = business_logic.process_order(mock_repo, order).await;
        assert!(result.is_ok());
    }
}
```

# 実装選択の判断基準

## プロジェクト規模による選択

### 小規模プロジェクト (< 10万行)
- **推奨**: Arc<Mutex>パターン
- **理由**: シンプルで学習コストが低い
- **注意点**: パフォーマンス要件の事前確認

### 中規模プロジェクト (10-50万行)
- **推奨**: Arc<Mutex> + 部分的にUnit of Work
- **理由**: 複雑性とパフォーマンスのバランス
- **注意点**: 設計一貫性の維持

### 大規模プロジェクト (50万行+)
- **推奨**: カスタムソリューション検討
- **理由**: 特定要件への最適化が必要
- **注意点**: 保守性と性能の両立

## チーム経験による選択

### Rust初心者中心
```rust
// シンプルなパターンに特化
// 学習曲線を考慮した設計
transaction_manager.simple_transaction(|tx| {
    repository.update(tx, data)
}).await?;
```

### Rust経験者中心
```rust
// 高度なパターンも積極採用
// 型安全性を最大限活用
transaction_manager
    .with_isolation(IsolationLevel::Serializable)
    .with_timeout(Duration::from_secs(30))
    .transaction(complex_business_logic)
    .await?;
```

## 長期保守性の考慮

### 技術負債の管理
- **ドキュメント化**: パターンの採用理由を明記
- **テストカバレッジ**: トランザクション境界の網羅
- **リファクタリング**: 定期的な設計見直し

### 技術進化への対応
```rust
// 将来的にasync traitが安定したら移行できる設計
#[async_trait]
pub trait FutureRepository {
    async fn find(&self, ctx: &TransactionContext, id: Id) -> Result<Entity>;
}
```

# まとめ: 実用的な選択基準

Rustにおけるトランザクション管理は、言語の特性を理解した上で適切なパターンを選択することが重要です。

## 推奨アプローチ

1. **まずArc<Mutex>から**: 多くのケースで十分な性能と安全性
2. **段階的に高度化**: 必要に応じてUnit of Work等を検討
3. **プロファイル駆動**: パフォーマンス問題が実際に起きてから最適化

## チーム導入のポイント

- **Rust習熟度**: チームの経験レベルに応じたパターン選択
- **段階的導入**: 小さく始めて徐々に拡大
- **継続的改善**: 運用経験を踏まえた設計見直し

## 技術的な学び

Rustのトランザクション管理を通じて、以下の重要な概念を実践的に学べます：

- **所有権システム**: 共有vs所有のトレードオフ
- **並行性**: Arc<Mutex>による安全な状態管理
- **抽象化**: traitを使った依存性逆転
- **エラーハンドリング**: Resultと?演算子の効果的活用

本記事で示したパターンは、実際のプロダクション環境での使用を前提として設計されています。ただし、プロジェクト固有の要件や制約に応じて、適切なカスタマイズを行うことを推奨します。

Rustでのトランザクション管理は決して簡単ではありませんが、適切なパターンを理解し段階的に導入することで、安全で保守性の高いアプリケーションを構築できます。ぜひ学習の出発点として、実際に手を動かして体験してみてください。

# 参考資料

- [Design the infrastructure persistence layer](https://learn.microsoft.com/en-us/dotnet/architecture/microservices/microservice-ddd-cqrs-patterns/infrastructure-persistence-layer-design)
- [Rust における Unit of Work の実装例](https://zenn.dev/poi2/articles/8162610d20798a)
- [SeaQL/sea-orm](https://github.com/SeaQL/sea-orm)
- [launchbadge/sqlx](https://github.com/launchbadge/sqlx)
- [Arc in std::sync](https://doc.rust-lang.org/std/sync/struct.Arc.html)
- [Mutex in std::sync](https://doc.rust-lang.org/std/sync/struct.Mutex.html)
- [The Rust Programming Language - Fearless Concurrency](https://doc.rust-lang.org/book/ch16-00-fearless-concurrency.html)