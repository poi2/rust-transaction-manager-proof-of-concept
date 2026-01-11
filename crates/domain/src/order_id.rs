use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OrderId(Uuid);

impl OrderId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn uuid(&self) -> &Uuid {
        &self.0
    }

    pub fn to_string(&self) -> String {
        self.0.to_string()
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