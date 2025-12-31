use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table((Alias::new("poc_for_sea_orm"), Order::Table))
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Order::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Order::ItemId).uuid().not_null())
                    .col(ColumnDef::new(Order::Quantity).integer().not_null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .table((Alias::new("poc_for_sea_orm"), Order::Table))
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum Order {
    Table,
    Id,
    ItemId,
    Quantity,
}