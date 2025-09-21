
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
