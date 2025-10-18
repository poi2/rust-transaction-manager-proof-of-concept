use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, Set};
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

use domain::db_context::DbContextMutexGuard;
use domain::todo_aggregate::Todo;
use domain::todo_repository::TodoRepositoryMutexGuard;

use crate::db_context::SeaOrmDbContextMutexGuard;
use crate::todo_entity::{ActiveModel, Column, Entity as TodoEntity};

/// SeaORM TodoRepository implementation using entity operations for type safety
#[derive(Clone)]
pub struct SeaOrmTodoRepositoryMutexGuard;

impl TodoRepositoryMutexGuard for SeaOrmTodoRepositoryMutexGuard {
    type DbContext = SeaOrmDbContextMutexGuard;
    type Error = anyhow::Error;

    async fn create(
        &self,
        db_context: &Arc<Mutex<Self::DbContext>>,
        id: Uuid,
        description: &str,
    ) -> Result<Todo, Self::Error> {
        let mut guard = db_context.lock().await;
        let txn = guard.get_transaction();

        // Using SeaORM ActiveModel for type-safe insert operations
        let todo = ActiveModel {
            id: Set(id),
            description: Set(description.to_string()),
        };

        todo.insert(txn).await?;
        Ok(Todo::new(id, description.to_string()))
    }

    async fn find_by_id(
        &self,
        db_context: &Arc<Mutex<Self::DbContext>>,
        id: Uuid,
    ) -> Result<Option<Todo>, Self::Error> {
        let mut guard = db_context.lock().await;
        let txn = guard.get_transaction();

        // Using SeaORM entity queries with type-safe column references
        let result = TodoEntity::find()
            .filter(Column::Id.eq(id))
            .one(txn)
            .await?;

        Ok(result.map(|model| Todo::new(model.id, model.description)))
    }

    async fn find_all(
        &self,
        db_context: &Arc<Mutex<Self::DbContext>>,
    ) -> Result<Vec<Todo>, Self::Error> {
        let mut guard = db_context.lock().await;
        let txn = guard.get_transaction();

        let models = TodoEntity::find()
            .order_by_asc(Column::Description)
            .all(txn)
            .await?;

        Ok(models
            .into_iter()
            .map(|model| Todo::new(model.id, model.description))
            .collect())
    }

    async fn update(
        &self,
        db_context: &Arc<Mutex<Self::DbContext>>,
        todo: Todo,
    ) -> Result<Todo, Self::Error> {
        let mut guard = db_context.lock().await;
        let txn = guard.get_transaction();

        // Using SeaORM ActiveModel for type-safe update operations
        let active_todo = ActiveModel {
            id: Set(todo.id()),
            description: Set(todo.description().to_string()),
        };

        active_todo.update(txn).await?;
        Ok(todo)
    }

    async fn delete(
        &self,
        db_context: &Arc<Mutex<Self::DbContext>>,
        id: Uuid,
    ) -> Result<(), Self::Error> {
        let mut guard = db_context.lock().await;
        let txn = guard.get_transaction();

        TodoEntity::delete_by_id(id).exec(txn).await?;
        Ok(())
    }
}
