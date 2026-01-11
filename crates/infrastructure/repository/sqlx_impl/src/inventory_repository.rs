use std::sync::Arc;

use domain::{
    db_context::DbContext,
    inventory::{aggregate::Inventory, repository::InventoryRepository},
    item::aggregate::ItemId,
};
use sqlx::Row;
use tokio::sync::Mutex;

use crate::SqlxDbContext;

#[derive(Clone)]
pub struct SqlxInventoryRepository;

impl InventoryRepository for SqlxInventoryRepository {
    type DbContext = SqlxDbContext;
    type Error = anyhow::Error;

    async fn find_by_item_id_for_update(
        &self,
        db_context: &Arc<Mutex<Self::DbContext>>,
        item_id: &ItemId,
    ) -> Result<Option<Inventory>, Self::Error> {
        let mut guard = db_context.lock().await;
        let txn = guard.get_transaction();

        let result = sqlx::query(
            "SELECT item_id, quantity FROM poc_for_sqlx.inventory WHERE item_id = $1 FOR UPDATE",
        )
        .bind(item_id.uuid())
        .fetch_optional(&mut **txn)
        .await?;

        match result {
            Some(row) => {
                let item_id: uuid::Uuid = row.try_get("item_id")?;
                let quantity: i32 = row.try_get("quantity")?;
                let inventory = Inventory::new(item_id.into(), quantity)?;
                Ok(Some(inventory))
            }
            None => Ok(None),
        }
    }

    async fn find_by_item_id(
        &self,
        db_context: &Arc<Mutex<Self::DbContext>>,
        item_id: &ItemId,
    ) -> Result<Option<Inventory>, Self::Error> {
        let mut guard = db_context.lock().await;
        let txn = guard.get_transaction();

        let result =
            sqlx::query("SELECT item_id, quantity FROM poc_for_sqlx.inventory WHERE item_id = $1")
                .bind(item_id.uuid())
                .fetch_optional(&mut **txn)
                .await?;

        match result {
            Some(row) => {
                let item_id: uuid::Uuid = row.try_get("item_id")?;
                let quantity: i32 = row.try_get("quantity")?;
                let inventory = Inventory::new(item_id.into(), quantity)?;
                Ok(Some(inventory))
            }
            None => Ok(None),
        }
    }

    async fn create(
        &self,
        db_context: &Arc<Mutex<Self::DbContext>>,
        inventory: Inventory,
    ) -> Result<Inventory, Self::Error> {
        let mut guard = db_context.lock().await;
        let txn = guard.get_transaction();

        sqlx::query("INSERT INTO poc_for_sqlx.inventory (item_id, quantity) VALUES ($1, $2)")
            .bind(inventory.item_id().uuid())
            .bind(inventory.quantity())
            .execute(&mut **txn)
            .await?;

        Ok(inventory)
    }

    async fn update(
        &self,
        db_context: &Arc<Mutex<Self::DbContext>>,
        inventory: Inventory,
    ) -> Result<Inventory, Self::Error> {
        let mut guard = db_context.lock().await;
        let txn = guard.get_transaction();

        sqlx::query("UPDATE poc_for_sqlx.inventory SET quantity = $1 WHERE item_id = $2")
            .bind(inventory.quantity())
            .bind(inventory.item_id().uuid())
            .execute(&mut **txn)
            .await?;

        Ok(inventory)
    }
}
