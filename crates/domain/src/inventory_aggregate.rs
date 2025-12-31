use crate::item_id::ItemId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Inventory {
    item_id: ItemId,
    quantity: i32,
}

impl Inventory {
    pub fn new(item_id: ItemId, quantity: i32) -> Result<Self, InventoryError> {
        if quantity < 0 {
            return Err(InventoryError::NegativeQuantity(quantity));
        }

        Ok(Self { item_id, quantity })
    }

    pub fn item_id(&self) -> &ItemId {
        &self.item_id
    }

    pub fn quantity(&self) -> i32 {
        self.quantity
    }

    pub fn decrease_stock(&mut self, amount: i32) -> Result<(), InventoryError> {
        if amount < 0 {
            return Err(InventoryError::NegativeAmount(amount));
        }

        if self.quantity < amount {
            return Err(InventoryError::InsufficientStock {
                available: self.quantity,
                requested: amount,
            });
        }

        self.quantity -= amount;
        Ok(())
    }

    pub fn increase_stock(&mut self, amount: i32) -> Result<(), InventoryError> {
        if amount < 0 {
            return Err(InventoryError::NegativeAmount(amount));
        }

        self.quantity += amount;
        Ok(())
    }
}

#[derive(Debug)]
pub enum InventoryError {
    NegativeQuantity(i32),
    NegativeAmount(i32),
    InsufficientStock { available: i32, requested: i32 },
}

impl std::fmt::Display for InventoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InventoryError::NegativeQuantity(quantity) => {
                write!(f, "Quantity cannot be negative: {}", quantity)
            }
            InventoryError::NegativeAmount(amount) => {
                write!(f, "Amount cannot be negative: {}", amount)
            }
            InventoryError::InsufficientStock { available, requested } => {
                write!(f, "Insufficient stock: available {}, requested {}", available, requested)
            }
        }
    }
}

impl std::error::Error for InventoryError {}