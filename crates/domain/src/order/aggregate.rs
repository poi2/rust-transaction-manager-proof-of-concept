use uuid::Uuid;

use crate::item::aggregate::ItemId;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OrderId(Uuid);

impl Default for OrderId {
    fn default() -> Self {
        Self::new()
    }
}

impl OrderId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl From<Uuid> for OrderId {
    fn from(id: Uuid) -> Self {
        Self(id)
    }
}

impl From<OrderId> for Uuid {
    fn from(id: OrderId) -> Self {
        id.0
    }
}

impl std::fmt::Display for OrderId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

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

impl TryFrom<CreateOrderCommand> for Order {
    type Error = OrderError;

    fn try_from(command: CreateOrderCommand) -> Result<Self, Self::Error> {
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
                write!(f, "Quantity must be positive: {quantity}")
            }
        }
    }
}

impl std::error::Error for OrderError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_order_success() {
        let order_id = OrderId::new();
        let item_id = ItemId::new();
        let order = Order::new(order_id.clone(), item_id.clone(), 5).unwrap();

        assert_eq!(order.id(), &order_id);
        assert_eq!(order.item_id(), &item_id);
        assert_eq!(order.quantity(), 5);
    }

    #[test]
    fn test_create_order_zero_quantity() {
        let order_id = OrderId::new();
        let item_id = ItemId::new();
        let result = Order::new(order_id, item_id, 0);

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            OrderError::InvalidQuantity(0)
        ));
    }

    #[test]
    fn test_create_order_negative_quantity() {
        let order_id = OrderId::new();
        let item_id = ItemId::new();
        let result = Order::new(order_id, item_id, -1);

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            OrderError::InvalidQuantity(-1)
        ));
    }

    #[test]
    fn test_order_from_command_success() {
        let item_id = ItemId::new();
        let command = CreateOrderCommand {
            item_id: item_id.clone(),
            quantity: 3,
        };

        let order = Order::try_from(command).unwrap();
        assert_eq!(order.item_id(), &item_id);
        assert_eq!(order.quantity(), 3);
    }

    #[test]
    fn test_order_from_command_invalid_quantity() {
        let item_id = ItemId::new();
        let command = CreateOrderCommand {
            item_id,
            quantity: -1,
        };

        let result = Order::try_from(command);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            OrderError::InvalidQuantity(-1)
        ));
    }

    #[test]
    fn test_order_id_conversion() {
        let uuid = Uuid::new_v4();
        let order_id = OrderId::from(uuid);

        assert_eq!(order_id.as_uuid(), &uuid);
        assert_eq!(Uuid::from(order_id.clone()), uuid);
    }
}
