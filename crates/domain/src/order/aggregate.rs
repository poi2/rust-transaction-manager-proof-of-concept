use uuid::Uuid;

use crate::item::aggregate::ItemId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Quantity(u32);

impl Quantity {
    pub fn new(value: u32) -> Result<Self, QuantityError> {
        if value == 0 {
            return Err(QuantityError::Zero);
        }
        Ok(Self(value))
    }

    pub fn value(&self) -> u32 {
        self.0
    }
}

impl TryFrom<i32> for Quantity {
    type Error = QuantityError;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        if value <= 0 {
            return Err(QuantityError::NonPositive(value));
        }
        Ok(Self(value as u32))
    }
}

impl From<Quantity> for i32 {
    fn from(q: Quantity) -> Self {
        q.0 as i32
    }
}

impl From<Quantity> for u32 {
    fn from(q: Quantity) -> Self {
        q.0
    }
}

impl std::fmt::Display for Quantity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QuantityError {
    Zero,
    NonPositive(i32),
}

impl std::fmt::Display for QuantityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QuantityError::Zero => write!(f, "Quantity must be positive"),
            QuantityError::NonPositive(value) => {
                write!(f, "Quantity must be positive: {value}")
            }
        }
    }
}

impl std::error::Error for QuantityError {}

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

    pub fn uuid(&self) -> &Uuid {
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
    quantity: Quantity,
}

impl Order {
    pub fn new(id: OrderId, item_id: ItemId, quantity: Quantity) -> Self {
        Self {
            id,
            item_id,
            quantity,
        }
    }

    pub fn id(&self) -> &OrderId {
        &self.id
    }

    pub fn item_id(&self) -> &ItemId {
        &self.item_id
    }

    pub fn quantity(&self) -> Quantity {
        self.quantity
    }
}

#[derive(Debug, Clone)]
pub struct CreateOrderCommand {
    pub item_id: ItemId,
    pub quantity: i32,
}

impl TryFrom<CreateOrderCommand> for Order {
    type Error = QuantityError;

    fn try_from(command: CreateOrderCommand) -> Result<Self, Self::Error> {
        let quantity = Quantity::try_from(command.quantity)?;
        Ok(Self::new(OrderId::new(), command.item_id, quantity))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_order_success() {
        let order_id = OrderId::new();
        let item_id = ItemId::new();
        let quantity = Quantity::new(5).unwrap();
        let order = Order::new(order_id.clone(), item_id.clone(), quantity);

        assert_eq!(order.id(), &order_id);
        assert_eq!(order.item_id(), &item_id);
        assert_eq!(order.quantity().value(), 5);
    }

    #[test]
    fn test_quantity_zero() {
        let result = Quantity::new(0);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), QuantityError::Zero));
    }

    #[test]
    fn test_quantity_from_i32_negative() {
        let result = Quantity::try_from(-1);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            QuantityError::NonPositive(-1)
        ));
    }

    #[test]
    fn test_quantity_from_i32_zero() {
        let result = Quantity::try_from(0);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), QuantityError::NonPositive(0)));
    }

    #[test]
    fn test_quantity_from_i32_positive() {
        let quantity = Quantity::try_from(5).unwrap();
        assert_eq!(quantity.value(), 5);
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
        assert_eq!(order.quantity().value(), 3);
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
            QuantityError::NonPositive(-1)
        ));
    }

    #[test]
    fn test_order_id_conversion() {
        let uuid = Uuid::new_v4();
        let order_id = OrderId::from(uuid);

        assert_eq!(order_id.uuid(), &uuid);
        assert_eq!(Uuid::from(order_id.clone()), uuid);
    }
}
