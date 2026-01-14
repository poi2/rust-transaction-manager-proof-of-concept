use std::sync::Arc;

use domain::{
    db_context::DbContext,
    inventory::{Inventory, InventoryRepository},
    item::ItemId,
};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QuerySelect, Set};
use tokio::sync::Mutex;

use crate::{
    db_context::SeaOrmDbContext,
    entities::inventory::{ActiveModel, Column, Entity},
};

#[derive(Clone)]
pub struct SeaOrmInventoryRepository;

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
            .filter(Column::ItemId.eq(*item_id.uuid()))
            .lock_exclusive()
            .one(txn)
            .await?;

        match result {
            Some(model) => {
                let inventory = Inventory::new(model.item_id.into(), model.quantity)?;
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

        let result = Entity::find()
            .filter(Column::ItemId.eq(*item_id.uuid()))
            .one(txn)
            .await?;

        match result {
            Some(model) => {
                let inventory = Inventory::new(model.item_id.into(), model.quantity)?;
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

        let active_model = ActiveModel {
            item_id: Set(*inventory.item_id().uuid()),
            quantity: Set(inventory.quantity()),
        };

        active_model.insert(txn).await?;
        Ok(inventory)
    }

    async fn update(
        &self,
        db_context: &Arc<Mutex<Self::DbContext>>,
        inventory: Inventory,
    ) -> Result<Inventory, Self::Error> {
        let mut guard = db_context.lock().await;
        let txn = guard.get_transaction();

        let active_model = ActiveModel {
            item_id: Set(*inventory.item_id().uuid()),
            quantity: Set(inventory.quantity()),
        };

        active_model.update(txn).await?;
        Ok(inventory)
    }
}
