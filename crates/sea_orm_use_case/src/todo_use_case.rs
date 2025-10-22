use anyhow::Result;
use domain::todo_aggregate::Todo;
use domain::todo_repository::TodoRepository;
use domain::transaction_manager::TransactionManager;
use uuid::Uuid;

pub struct TodoUseCase<TM, TR> {
    transaction_manager: TM,
    todo_repository: TR,
}

impl<TM, TR> TodoUseCase<TM, TR>
where
    TM: TransactionManager + Clone,
    TR: TodoRepository + Clone,
{
    pub fn new(transaction_manager: TM, todo_repository: TR) -> Self {
        Self {
            transaction_manager,
            todo_repository,
        }
    }

    pub async fn create_todo(&self, description: &str) -> Result<Todo> {
        let todo_id = Uuid::new_v4();
        let todo_repository = self.todo_repository.clone();
        
        self.transaction_manager
            .transaction(|db_context| async move {
                todo_repository
                    .create(&db_context, todo_id, description)
                    .await
            })
            .await
    }

    pub async fn create_multiple_todos_and_get_all(
        &self,
        descriptions: Vec<&str>,
    ) -> Result<(Vec<Todo>, Vec<Todo>)> {
        let todo_repository = self.todo_repository.clone();
        
        self.transaction_manager
            .transaction(|db_context| async move {
                let mut created_todos = Vec::new();
                
                for description in descriptions {
                    let todo_id = Uuid::new_v4();
                    let todo = todo_repository
                        .create(&db_context, todo_id, description)
                        .await?;
                    created_todos.push(todo);
                }

                let all_todos = todo_repository.find_all(&db_context).await?;
                
                Ok((created_todos, all_todos))
            })
            .await
    }

    pub async fn find_todo_by_id(&self, todo_id: Uuid) -> Result<Option<Todo>> {
        let todo_repository = self.todo_repository.clone();
        
        self.transaction_manager
            .transaction(|db_context| async move {
                todo_repository.find_by_id(&db_context, todo_id).await
            })
            .await
    }

    pub async fn update_todo(&self, mut todo: Todo, new_description: String) -> Result<Todo> {
        let todo_repository = self.todo_repository.clone();
        
        self.transaction_manager
            .transaction(|db_context| async move {
                todo.update_description(new_description);
                todo_repository.update(&db_context, todo).await
            })
            .await
    }
}