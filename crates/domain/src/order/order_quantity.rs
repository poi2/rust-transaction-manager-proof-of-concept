#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OrderQuantity(u32);

impl OrderQuantity {
    pub fn new(value: u32) -> Result<Self, OrderQuantityError> {
        if value == 0 {
            return Err(OrderQuantityError::Zero);
        }
        Ok(Self(value))
    }

    pub fn value(&self) -> u32 {
        self.0
    }
}

impl TryFrom<i32> for OrderQuantity {
    type Error = OrderQuantityError;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        if value <= 0 {
            return Err(OrderQuantityError::NonPositive(value));
        }
        Ok(Self(value as u32))
    }
}

impl From<OrderQuantity> for i32 {
    fn from(q: OrderQuantity) -> Self {
        q.0 as i32
    }
}

impl From<OrderQuantity> for u32 {
    fn from(q: OrderQuantity) -> Self {
        q.0
    }
}

impl std::fmt::Display for OrderQuantity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrderQuantityError {
    Zero,
    NonPositive(i32),
}

impl std::fmt::Display for OrderQuantityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OrderQuantityError::Zero => write!(f, "Order quantity must be positive"),
            OrderQuantityError::NonPositive(value) => {
                write!(f, "Order quantity must be positive: {value}")
            }
        }
    }
}

impl std::error::Error for OrderQuantityError {}
