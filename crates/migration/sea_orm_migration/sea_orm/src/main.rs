use migration::{Migrator, MigratorTrait};
use sea_orm::{Database, DbErr};

#[tokio::main]
async fn main() -> Result<(), DbErr> {
    let database_url = "postgres://postgres:password@localhost:5432/poc_transaction_manager";
    let db = Database::connect(database_url).await?;

    println!("Running SeaORM migrations for poc_for_sea_orm database...");
    Migrator::up(&db, None).await?;
    println!("Migrations completed successfully!");

    Ok(())
}