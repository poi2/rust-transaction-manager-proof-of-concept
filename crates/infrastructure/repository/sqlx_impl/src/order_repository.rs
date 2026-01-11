use std::sync::Arc;

use domain::{
    db_context::DbContext,
    order::{
        aggregate::{Order, OrderId},
        repository::OrderRepository,
    },
};
use sqlx::Row;
use tokio::sync::Mutex;

use crate::SqlxDbContext;

#[derive(Clone)]
pub struct SqlxOrderRepository;

impl OrderRepository for SqlxOrderRepository {
    type DbContext = SqlxDbContext;
    type Error = anyhow::Error;

    async fn find_by_id(
        &self,
        db_context: &Arc<Mutex<Self::DbContext>>,
        id: &OrderId,
    ) -> Result<Option<Order>, Self::Error> {
        let mut guard = db_context.lock().await;
        let txn = guard.get_transaction();

        let result =
            sqlx::query("SELECT id, item_id, quantity FROM poc_for_sqlx.orders WHERE id = $1")
                .bind(id.uuid())
                .fetch_optional(&mut **txn)
                .await?;

        match result {
            Some(row) => {
                let id: uuid::Uuid = row.try_get("id")?;
                let item_id: uuid::Uuid = row.try_get("item_id")?;
                let quantity: i32 = row.try_get("quantity")?;
                let order = Order::new(id.into(), item_id.into(), quantity)?;
                Ok(Some(order))
            }
            None => Ok(None),
        }
    }

    async fn create(
        &self,
        db_context: &Arc<Mutex<Self::DbContext>>,
        order: Order,
    ) -> Result<Order, Self::Error> {
        let mut guard = db_context.lock().await;
        let txn = guard.get_transaction();

        sqlx::query("INSERT INTO poc_for_sqlx.orders (id, item_id, quantity) VALUES ($1, $2, $3)")
            .bind(order.id().uuid())
            .bind(order.item_id().uuid())
            .bind(order.quantity())
            .execute(&mut **txn)
            .await?;

        Ok(order)
    }
}
