use std::sync::Arc;

use domain::{
    db_context::DbContext,
    order::{
        aggregate::{Order, OrderId},
        repository::OrderRepository,
    },
};
// SeaORM entity definitions
use sea_orm::entity::prelude::*;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use tokio::sync::Mutex;

use crate::db_context::SeaOrmDbContext;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(schema_name = "poc_for_sea_orm", table_name = "orders")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    pub item_id: Uuid,
    pub quantity: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

#[derive(Clone)]
pub struct SeaOrmOrderRepository;

impl OrderRepository for SeaOrmOrderRepository {
    type DbContext = SeaOrmDbContext;
    type Error = anyhow::Error;

    async fn find_by_id(
        &self,
        db_context: &Arc<Mutex<Self::DbContext>>,
        id: &OrderId,
    ) -> Result<Option<Order>, Self::Error> {
        let mut guard = db_context.lock().await;
        let txn = guard.get_transaction();

        let result = Entity::find()
            .filter(Column::Id.eq(*id.uuid()))
            .one(txn)
            .await?;

        match result {
            Some(model) => {
                let order = Order::new(model.id.into(), model.item_id.into(), model.quantity)?;
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

        let active_model = ActiveModel {
            id: Set(*order.id().uuid()),
            item_id: Set(*order.item_id().uuid()),
            quantity: Set(order.quantity()),
        };

        active_model.insert(txn).await?;
        Ok(order)
    }
}
