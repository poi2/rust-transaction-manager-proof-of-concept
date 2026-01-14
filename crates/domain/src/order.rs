#[allow(clippy::module_inception)]
pub mod order;
pub mod order_id;
pub mod order_quantity;
pub mod order_repository;

pub use order::{CreateOrderCommand, Order};
pub use order_id::OrderId;
pub use order_quantity::{OrderQuantity, OrderQuantityError};
pub use order_repository::OrderRepository;
