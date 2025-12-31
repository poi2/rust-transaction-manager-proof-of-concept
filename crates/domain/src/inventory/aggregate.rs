use crate::item::aggregate::ItemId;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_inventory_success() {
        let item_id = ItemId::new();
        let inventory = Inventory::new(item_id.clone(), 10).unwrap();
        
        assert_eq!(inventory.item_id(), &item_id);
        assert_eq!(inventory.quantity(), 10);
    }

    #[test]
    fn test_create_inventory_negative_quantity() {
        let item_id = ItemId::new();
        let result = Inventory::new(item_id, -1);
        
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), InventoryError::NegativeQuantity(-1)));
    }

    #[test]
    fn test_decrease_stock_success() {
        let item_id = ItemId::new();
        let mut inventory = Inventory::new(item_id, 10).unwrap();
        
        inventory.decrease_stock(3).unwrap();
        assert_eq!(inventory.quantity(), 7);
    }

    #[test]
    fn test_decrease_stock_insufficient() {
        let item_id = ItemId::new();
        let mut inventory = Inventory::new(item_id, 5).unwrap();
        
        let result = inventory.decrease_stock(10);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(), 
            InventoryError::InsufficientStock { available: 5, requested: 10 }
        ));
    }

    #[test]
    fn test_decrease_stock_negative_amount() {
        let item_id = ItemId::new();
        let mut inventory = Inventory::new(item_id, 10).unwrap();
        
        let result = inventory.decrease_stock(-1);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), InventoryError::NegativeAmount(-1)));
    }

    #[test]
    fn test_increase_stock_success() {
        let item_id = ItemId::new();
        let mut inventory = Inventory::new(item_id, 10).unwrap();
        
        inventory.increase_stock(5).unwrap();
        assert_eq!(inventory.quantity(), 15);
    }

    #[test]
    fn test_increase_stock_negative_amount() {
        let item_id = ItemId::new();
        let mut inventory = Inventory::new(item_id, 10).unwrap();
        
        let result = inventory.increase_stock(-1);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), InventoryError::NegativeAmount(-1)));
    }
}