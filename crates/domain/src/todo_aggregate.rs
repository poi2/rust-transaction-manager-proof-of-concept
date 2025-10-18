use uuid::Uuid;

/// Todo aggregate root
#[derive(Debug, Clone, PartialEq)]
pub struct Todo {
    pub id: Uuid,
    pub description: String,
}

impl Todo {
    pub fn new(id: Uuid, description: String) -> Self {
        Self { id, description }
    }

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn update_description(&mut self, description: String) {
        self.description = description;
    }
}
