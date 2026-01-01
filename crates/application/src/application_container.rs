use std::sync::Arc;
use use_case::order_management::OrderManagementUseCase;

pub struct ApplicationContainer<TM, IR, OR> {
    pub order_management_use_case: OrderManagementUseCase<TM, IR, OR>,
}

impl
    ApplicationContainer<
        sea_orm_impl::SeaOrmTransactionManager,
        sea_orm_impl::SeaOrmInventoryRepository,
        sea_orm_impl::SeaOrmOrderRepository,
    >
{
    pub async fn new_with_sea_orm(database_url: &str) -> Result<Self, anyhow::Error> {
        let transaction_manager =
            Arc::new(sea_orm_impl::SeaOrmTransactionManager::new(database_url).await?);
        let inventory_repository = Arc::new(sea_orm_impl::SeaOrmInventoryRepository);
        let order_repository = Arc::new(sea_orm_impl::SeaOrmOrderRepository);

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

impl
    ApplicationContainer<
        sqlx_impl::SqlxTransactionManager,
        sqlx_impl::SqlxInventoryRepository,
        sqlx_impl::SqlxOrderRepository,
    >
{
    pub async fn new_with_sqlx(database_url: &str) -> Result<Self, anyhow::Error> {
        let pool = sqlx::PgPool::connect(database_url).await?;
        let transaction_manager = Arc::new(sqlx_impl::SqlxTransactionManager::new(pool));
        let inventory_repository = Arc::new(sqlx_impl::SqlxInventoryRepository);
        let order_repository = Arc::new(sqlx_impl::SqlxOrderRepository);

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
