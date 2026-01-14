#[cfg(test)]
mod integration_tests {

    use domain::{
        inventory::{Inventory, InventoryQuantity},
        item::ItemId,
        order::{CreateOrderCommand, Order},
    };

    // Note: These are integration tests that require Docker and would be more complex
    // For now, we'll write unit tests that focus on the repository logic

    #[tokio::test]
    async fn test_domain_aggregate_integration() {
        // Test that our domain aggregates work together properly
        let item_id = ItemId::new();

        // Create inventory
        let mut inventory = Inventory::new(item_id.clone(), 10).unwrap();
        assert_eq!(inventory.quantity(), 10);

        // Create order
        let command = CreateOrderCommand {
            item_id: item_id.clone(),
            quantity: 3,
        };
        let order = Order::try_from(command).unwrap();

        // Simulate inventory update
        let inventory_quantity = InventoryQuantity::try_from(i32::from(order.quantity())).unwrap();
        inventory.decrease_stock(inventory_quantity).unwrap();
        assert_eq!(inventory.quantity(), 7);
    }

    #[tokio::test]
    async fn test_insufficient_stock_scenario() {
        let item_id = ItemId::new();
        let mut inventory = Inventory::new(item_id.clone(), 2).unwrap();

        let command = CreateOrderCommand {
            item_id,
            quantity: 5,
        };
        let order = Order::try_from(command).unwrap();

        // Should fail due to insufficient stock
        let inventory_quantity = InventoryQuantity::try_from(i32::from(order.quantity())).unwrap();
        let result = inventory.decrease_stock(inventory_quantity);
        assert!(result.is_err());
    }
}
