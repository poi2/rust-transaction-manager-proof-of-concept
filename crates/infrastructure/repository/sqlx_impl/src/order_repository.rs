use std::sync::Arc;

use domain::{
    db_context::DbContext,
    item::aggregate::ItemId,
    order::{
        aggregate::{Order, OrderId},
        repository::OrderRepository,
    },
};
use sqlx::Row;
use tokio::sync::Mutex;

use crate::transaction_manager::OwnedSqlxDbContext;

#[derive(Clone)]
pub struct SqlxOrderRepository;

impl OrderRepository for SqlxOrderRepository {
    type DbContext = OwnedSqlxDbContext;
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
                .bind(id.as_uuid())
                .fetch_optional(&mut **txn)
                .await?;

        match result {
            Some(row) => {
                let id: uuid::Uuid = row.try_get("id")?;
                let item_id: uuid::Uuid = row.try_get("item_id")?;
                let quantity: i32 = row.try_get("quantity")?;
                let order =
                    Order::new(OrderId::from_uuid(id), ItemId::from_uuid(item_id), quantity)?;
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
            .bind(order.id().as_uuid())
            .bind(order.item_id().as_uuid())
            .bind(order.quantity())
            .execute(&mut **txn)
            .await?;

        Ok(order)
    }
}
