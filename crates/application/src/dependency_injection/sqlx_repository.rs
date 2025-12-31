use std::sync::Arc;

use infrastructure::repository::sqlx_impl::{
    transaction_manager_v1::SqlxTransactionManagerV1, SqlxInventoryRepository, SqlxOrderRepository,
};
use use_case::order_management::OrderManagementUseCase;

pub type SqlxOrderManagementUseCase =
    OrderManagementUseCase<SqlxTransactionManagerV1, SqlxInventoryRepository, SqlxOrderRepository>;

pub struct SqlxDependencyInjection {
    pub order_management_use_case: SqlxOrderManagementUseCase,
}

impl SqlxDependencyInjection {
    pub async fn new(database_url: &str) -> Result<Self, anyhow::Error> {
        let pool = sqlx::PgPool::connect(database_url).await?;
        let transaction_manager = Arc::new(SqlxTransactionManagerV1::new(pool));
        let inventory_repository = Arc::new(SqlxInventoryRepository);
        let order_repository = Arc::new(SqlxOrderRepository);

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
