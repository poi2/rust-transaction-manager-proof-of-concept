use futures::future::BoxFuture;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, Set};
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

use domain::{db_context::DbContext, todo_aggregate::Todo, todo_repository::TodoRepository};

use crate::{
    db_context::SeaOrmDbContext,
    todo_entity::{ActiveModel, Column, Entity as TodoEntity},
};

/// SeaORM TodoRepository implementation using entity operations for type safety
#[derive(Clone)]
pub struct SeaOrmTodoRepository;

impl TodoRepository for SeaOrmTodoRepository {
    type DbContext = SeaOrmDbContext;
    type Error = anyhow::Error;

    fn create(
        &self,
        db_context: &Arc<Mutex<Self::DbContext>>,
        id: Uuid,
        description: &str,
    ) -> BoxFuture<'_, Result<Todo, Self::Error>> {
        // TODO: Question ここで clone しちゃって大丈夫なものか？
        let db_context = db_context.clone();
        let description = description.to_string();
        Box::pin(async move {
            let mut guard = db_context.lock().await;
            let txn = guard.get_transaction();

            // Using SeaORM ActiveModel for type-safe insert operations
            let todo = ActiveModel {
                id: Set(id),
                description: Set(description.clone()),
            };

            todo.insert(txn).await?;
            Ok(Todo::new(id, description))
        })
    }

    fn find_by_id(
        &self,
        db_context: &Arc<Mutex<Self::DbContext>>,
        id: Uuid,
    ) -> BoxFuture<'_, Result<Option<Todo>, Self::Error>> {
        let db_context = db_context.clone();
        Box::pin(async move {
            let mut guard = db_context.lock().await;
            let txn = guard.get_transaction();

            // Using SeaORM entity queries with type-safe column references
            let result = TodoEntity::find()
                .filter(Column::Id.eq(id))
                .one(txn)
                .await?;

            Ok(result.map(|model| Todo::new(model.id, model.description)))
        })
    }

    fn find_all(
        &self,
        db_context: &Arc<Mutex<Self::DbContext>>,
    ) -> BoxFuture<'_, Result<Vec<Todo>, Self::Error>> {
        let db_context = db_context.clone();
        Box::pin(async move {
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
        })
    }

    fn update(
        &self,
        db_context: &Arc<Mutex<Self::DbContext>>,
        todo: Todo,
    ) -> BoxFuture<'_, Result<Todo, Self::Error>> {
        let db_context = db_context.clone();
        Box::pin(async move {
            let mut guard = db_context.lock().await;
            let txn = guard.get_transaction();

            // Using SeaORM ActiveModel for type-safe update operations
            let active_todo = ActiveModel {
                id: Set(todo.id()),
                description: Set(todo.description().to_string()),
            };

            active_todo.update(txn).await?;
            Ok(todo)
        })
    }

    fn delete(
        &self,
        db_context: &Arc<Mutex<Self::DbContext>>,
        id: Uuid,
    ) -> BoxFuture<'_, Result<(), Self::Error>> {
        let db_context = db_context.clone();
        Box::pin(async move {
            let mut guard = db_context.lock().await;
            let txn = guard.get_transaction();

            TodoEntity::delete_by_id(id).exec(txn).await?;
            Ok(())
        })
    }
}
