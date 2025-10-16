
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
