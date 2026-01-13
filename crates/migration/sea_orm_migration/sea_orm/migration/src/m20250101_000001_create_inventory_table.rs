use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create schema if not exists
        manager
            .create_schema(
                Schema::create()
                    .schema(Alias::new("poc_for_sea_orm"))
                    .if_not_exists()
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table((Alias::new("poc_for_sea_orm"), Inventory::Table))
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Inventory::ItemId)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Inventory::Quantity).integer().not_null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .table((Alias::new("poc_for_sea_orm"), Inventory::Table))
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum Inventory {
    Table,
    ItemId,
    Quantity,
}
