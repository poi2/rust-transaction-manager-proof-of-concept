use std::{future::Future, sync::Arc};

use tokio::sync::Mutex;

use crate::{
    db_context::DbContext,
    order::aggregate::{Order, OrderId},
};

pub trait OrderRepository: Send + Sync {
    type DbContext: DbContext;
    type Error: Send + Sync + 'static;

    fn find_by_id(
        &self,
        db_context: &Arc<Mutex<Self::DbContext>>,
        id: &OrderId,
    ) -> impl Future<Output = Result<Option<Order>, Self::Error>> + Send
    where
        Self: Send;

    fn create(
        &self,
        db_context: &Arc<Mutex<Self::DbContext>>,
        order: Order,
    ) -> impl Future<Output = Result<Order, Self::Error>> + Send
    where
        Self: Send;
}
