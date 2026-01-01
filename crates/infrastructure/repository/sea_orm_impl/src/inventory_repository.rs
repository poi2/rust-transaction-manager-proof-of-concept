use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::db_context::SeaOrmDbContext;
use domain::{
    db_context::DbContext,
    inventory::{aggregate::Inventory, repository::InventoryRepository},
    item::aggregate::ItemId,
};

// SeaORM entity definitions (would typically be in a separate entities module)
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(schema_name = "poc_for_sea_orm", table_name = "inventory")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub item_id: Uuid,
    pub quantity: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

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
            .filter(Column::ItemId.eq(*item_id.as_uuid()))
            .one(txn)
            .await?;

        match result {
            Some(model) => {
                let inventory = Inventory::new(ItemId::from_uuid(model.item_id), model.quantity)?;
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
            .filter(Column::ItemId.eq(*item_id.as_uuid()))
            .one(txn)
            .await?;

        match result {
            Some(model) => {
                let inventory = Inventory::new(ItemId::from_uuid(model.item_id), model.quantity)?;
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
            item_id: Set(*inventory.item_id().as_uuid()),
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
            item_id: Set(*inventory.item_id().as_uuid()),
            quantity: Set(inventory.quantity()),
        };

        active_model.update(txn).await?;
        Ok(inventory)
    }
}
