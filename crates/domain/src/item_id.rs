use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ItemId(Uuid);

impl ItemId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_uuid(id: Uuid) -> Self {
        Self(id)
    }

    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }

    pub fn to_string(&self) -> String {
        self.0.to_string()
    }
}

impl From<Uuid> for ItemId {
    fn from(id: Uuid) -> Self {
        Self(id)
    }
}

impl From<ItemId> for Uuid {
    fn from(id: ItemId) -> Self {
        id.0
    }
}

impl std::fmt::Display for ItemId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}