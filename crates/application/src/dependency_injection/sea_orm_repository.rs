use std::sync::Arc;

use infrastructure::repository::sea_orm_impl::{
    SeaOrmInventoryRepository, SeaOrmOrderRepository, SeaOrmTransactionManager,
};
use use_case::order_management::OrderManagementUseCase;

pub type SeaOrmOrderManagementUseCase = OrderManagementUseCase<
    SeaOrmTransactionManager,
    SeaOrmInventoryRepository,
    SeaOrmOrderRepository,
>;

pub struct SeaOrmDependencyInjection {
    pub order_management_use_case: SeaOrmOrderManagementUseCase,
}

impl SeaOrmDependencyInjection {
    pub async fn new(database_url: &str) -> Result<Self, anyhow::Error> {
        let transaction_manager = Arc::new(SeaOrmTransactionManager::new(database_url).await?);
        let inventory_repository = Arc::new(SeaOrmInventoryRepository);
        let order_repository = Arc::new(SeaOrmOrderRepository);

        let order_management_use_case = OrderManagementUseCase::new(
            transaction_manager,
            inventory_repository,
            order_repository,
        );

        Ok(Self {
            order_management_use_case,
        })
    }
}
