use crate::{item_id::ItemId, order_id::OrderId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Order {
    id: OrderId,
    item_id: ItemId,
    quantity: i32,
}

impl Order {
    pub fn new(id: OrderId, item_id: ItemId, quantity: i32) -> Result<Self, OrderError> {
        if quantity <= 0 {
            return Err(OrderError::InvalidQuantity(quantity));
        }

        Ok(Self {
            id,
            item_id,
            quantity,
        })
    }

    pub fn id(&self) -> &OrderId {
        &self.id
    }

    pub fn item_id(&self) -> &ItemId {
        &self.item_id
    }

    pub fn quantity(&self) -> i32 {
        self.quantity
    }
}

#[derive(Debug, Clone)]
pub struct CreateOrderCommand {
    pub item_id: ItemId,
    pub quantity: i32,
}

impl Order {
    pub fn from(command: CreateOrderCommand) -> Result<Self, OrderError> {
        Self::new(OrderId::new(), command.item_id, command.quantity)
    }
}

#[derive(Debug)]
pub enum OrderError {
    InvalidQuantity(i32),
}

impl std::fmt::Display for OrderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OrderError::InvalidQuantity(quantity) => {
                write!(f, "Quantity must be positive: {}", quantity)
            }
        }
    }
}

impl std::error::Error for OrderError {}