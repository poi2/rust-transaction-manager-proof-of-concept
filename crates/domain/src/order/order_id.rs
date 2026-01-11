use uuid::Uuid;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_order_id_conversion() {
        let uuid = Uuid::new_v4();
        let order_id = OrderId::from(uuid);

        assert_eq!(order_id.uuid(), &uuid);
        assert_eq!(Uuid::from(order_id.clone()), uuid);
    }
}
