
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
