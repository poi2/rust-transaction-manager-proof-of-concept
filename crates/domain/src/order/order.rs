use super::{
    order_id::OrderId,
    order_quantity::{OrderQuantity, OrderQuantityError},
};
use crate::item::ItemId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Order {
    id: OrderId,
    item_id: ItemId,
    quantity: OrderQuantity,
}

impl Order {
    pub fn new(id: OrderId, item_id: ItemId, quantity: OrderQuantity) -> Self {
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

    pub fn quantity(&self) -> OrderQuantity {
        self.quantity
    }
}

#[derive(Debug, Clone)]
pub struct CreateOrderCommand {
    pub item_id: ItemId,
    pub quantity: i32,
}

impl TryFrom<CreateOrderCommand> for Order {
    type Error = OrderQuantityError;

    fn try_from(command: CreateOrderCommand) -> Result<Self, Self::Error> {
        let quantity = OrderQuantity::try_from(command.quantity)?;
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
        let quantity = OrderQuantity::new(5).unwrap();
        let order = Order::new(order_id.clone(), item_id.clone(), quantity);

        assert_eq!(order.id(), &order_id);
        assert_eq!(order.item_id(), &item_id);
        assert_eq!(order.quantity().value(), 5);
    }

    #[test]
    fn test_quantity_zero() {
        let result = OrderQuantity::new(0);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), OrderQuantityError::Zero));
    }

    #[test]
    fn test_quantity_from_i32_negative() {
        let result = OrderQuantity::try_from(-1);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            OrderQuantityError::NonPositive(-1)
        ));
    }

    #[test]
    fn test_quantity_from_i32_zero() {
        let result = OrderQuantity::try_from(0);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            OrderQuantityError::NonPositive(0)
        ));
    }

    #[test]
    fn test_quantity_from_i32_positive() {
        let quantity = OrderQuantity::try_from(5).unwrap();
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
            OrderQuantityError::NonPositive(-1)
        ));
    }
}
