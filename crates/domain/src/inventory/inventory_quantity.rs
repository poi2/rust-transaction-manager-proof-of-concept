#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InventoryQuantity(u32);

impl InventoryQuantity {
    pub fn new(value: u32) -> Result<Self, InventoryQuantityError> {
        if value == 0 {
            return Err(InventoryQuantityError::Zero);
        }
        Ok(Self(value))
    }

    pub fn value(&self) -> u32 {
        self.0
    }
}

impl TryFrom<i32> for InventoryQuantity {
    type Error = InventoryQuantityError;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        if value <= 0 {
            return Err(InventoryQuantityError::NonPositive(value));
        }
        Ok(Self(value as u32))
    }
}

impl From<InventoryQuantity> for i32 {
    fn from(q: InventoryQuantity) -> Self {
        q.0 as i32
    }
}

impl From<InventoryQuantity> for u32 {
    fn from(q: InventoryQuantity) -> Self {
        q.0
    }
}

impl std::fmt::Display for InventoryQuantity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InventoryQuantityError {
    Zero,
    NonPositive(i32),
}

impl std::fmt::Display for InventoryQuantityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InventoryQuantityError::Zero => write!(f, "Inventory quantity must be positive"),
            InventoryQuantityError::NonPositive(value) => {
                write!(f, "Inventory quantity must be positive: {value}")
            }
        }
    }
}

impl std::error::Error for InventoryQuantityError {}
