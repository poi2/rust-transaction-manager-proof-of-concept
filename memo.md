
こういうことがやれたら良い。

```rs
context.transaction(move |acquire| {
    todo_repository(acquire, new_todo).await?;
})

todo_repository(context.pool(), new_todo).await?;
```

https://docs.rs/sqlx/latest/sqlx/trait.Acquire.html
https://docs.rs/sqlx/latest/sqlx/struct.Transaction.html

context が pool を持ち、

fn <T, E> transaction(&self, func: function) -> Result<T, E> {
    let tx = self.pool.begin();

    let returning = func(tx);

    tx.commit();

    Ok(returning)
}

ということができるとよいな。

とても参考になった記事。
https://zenn.dev/penysho/articles/a48ca73b757656#%E3%83%88%E3%83%A9%E3%83%B3%E3%82%B6%E3%82%AF%E3%82%B7%E3%83%A7%E3%83%B3%E3%83%9F%E3%83%89%E3%83%AB%E3%82%A6%E3%82%A7%E3%82%A2%E3%81%AE%E5%AE%9F%E8%A3%85

-----

## 2025-10-16 MutexGuard 版

https://doc.rust-jp.rs/book-ja/ch16-03-shared-state.html
MutexGuard を使うと clone できない object を &mut 状態で別 fn にわたす事ができる。

usecase の中で

```rust
self.transaction_manager.transaction(|db_context| async move {
    // 1 回目の呼び出し
    let returning_todo1 = self.todo_repository.create(&db_context, todo1).await?;
    // 2 回目の呼び出し
    let returning_todo2 = self.todo_repository.create(&db_context, todo2).await?;

    vec![returning_todo1, returning_todo2]
}).await?;
```

という書き方を実現したい。

transaction_manager の transaction fn は DbContext を内部で生成する。
DbContext には transaction が入っている。これは基本 Clone できない。これを MutexGuard で包んでおきます。
transaction fn は FnOnce を引数で受け取り、FnOnce に Mutex<DbContext> を渡す。

usecase の 1 回目、2 回目の呼び出しでは Mutex<DbContext> を渡すことになります。
そのとき、ToDoRepository の create fn の中で DbContext get_transaction をすると transaction を呼び出せるようにする。

DbContext は Generic に Transaction の型を指定できるようにする。
DbContext の trait は domain に書かれており、SqlxDbContext が infrastructure に書かれている。SqlxDbContext では postgres の transaction 型として定義する。

既存のコードと見分けがつくように TransactionManagerMutexGuard, DbContextMutexGuard というように suffix に MutexGuard を付けて実装してください。

- 実装
- テスト
    - unit test
    - usecase の中で使うパターンが実装できることの検証

を書いてください。

- compile error はすべて解消する
- test はすべて通す
- fmt, clippy はすべて通す

を守ってください。

----

2025-12-30
いったん、不要なブロックを移動させた。

```md
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
```

# 2025-12-31

- TODO: error handling は anyhow に寄せてしまう。本論はトランザクションの管理でありエラーではないから。
