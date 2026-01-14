#[allow(clippy::module_inception)]
pub mod inventory;
pub mod inventory_quantity;
pub mod inventory_repository;

pub use inventory::{Inventory, InventoryError};
pub use inventory_quantity::{InventoryQuantity, InventoryQuantityError};
pub use inventory_repository::InventoryRepository;
