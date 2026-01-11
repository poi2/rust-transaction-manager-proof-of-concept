use std::{future::Future, sync::Arc};

use tokio::sync::Mutex;

use crate::{db_context::DbContext, inventory::Inventory, item::ItemId};

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

    fn find_by_item_id(
        &self,
        db_context: &Arc<Mutex<Self::DbContext>>,
        item_id: &ItemId,
    ) -> impl Future<Output = Result<Option<Inventory>, Self::Error>> + Send
    where
        Self: Send;

    fn create(
        &self,
        db_context: &Arc<Mutex<Self::DbContext>>,
        inventory: Inventory,
    ) -> impl Future<Output = Result<Inventory, Self::Error>> + Send
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
