pub use sea_orm_migration::prelude::*;

mod m20250101_000001_create_inventory_table;
mod m20250101_000002_create_order_table;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20250101_000001_create_inventory_table::Migration),
            Box::new(m20250101_000002_create_order_table::Migration),
        ]
    }
}
