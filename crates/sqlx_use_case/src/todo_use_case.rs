use anyhow::Result;
use domain::todo_aggregate::Todo;
use domain::todo_repository::TodoRepository;
use domain::transaction_manager::TransactionManager;
use std::future::Future;
use std::pin::Pin;
use uuid::Uuid;

pub struct TodoUseCase<TM, TR> {
    transaction_manager: TM,
    todo_repository: TR,
}

impl<TM, TR> TodoUseCase<TM, TR> {
    pub fn new(transaction_manager: TM, todo_repository: TR) -> Self {
        Self {
            transaction_manager,
            todo_repository,
        }
    }
}

impl<TM, TR> TodoUseCase<TM, TR>
where
    TM: TransactionManager,
    TR: TodoRepository<DbContext = TM::DbContext, Error = TM::Error> + Clone,
    TM::Error: std::error::Error + 'static,
{
    pub async fn create_todo(&self, description: &str) -> Result<Todo, Box<dyn std::error::Error>> {
        let todo_id = Uuid::new_v4();
        let todo_repository = self.todo_repository.clone();

        let result = self
            .transaction_manager
            .transaction(
                |db_context| -> Pin<Box<dyn Future<Output = Result<Todo, TM::Error>> + Send>> {
                    let todo_repository = todo_repository.clone();
                    Box::pin(async move {
                        todo_repository
                            .create(&db_context, todo_id, description)
                            .await
                    })
                },
            )
            .await;

        result.map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
    }

    pub async fn create_multiple_todos_and_get_all(
        &self,
        descriptions: Vec<&str>,
    ) -> Result<(Vec<Todo>, Vec<Todo>), Box<dyn std::error::Error>> {
        let todo_repository = self.todo_repository.clone();

        let result = self
            .transaction_manager
            .transaction(
                |db_context| -> Pin<
                    Box<dyn Future<Output = Result<(Vec<Todo>, Vec<Todo>), TM::Error>> + Send>,
                > {
                    let todo_repository = todo_repository.clone();
                    Box::pin(async move {
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
                },
            )
            .await;

        result.map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
    }

    pub async fn find_todo_by_id(
        &self,
        todo_id: Uuid,
    ) -> Result<Option<Todo>, Box<dyn std::error::Error>> {
        let todo_repository = self.todo_repository.clone();

        let result = self.transaction_manager
            .transaction(|db_context| -> Pin<Box<dyn Future<Output = Result<Option<Todo>, TM::Error>> + Send>> {
                let todo_repository = todo_repository.clone();
                Box::pin(async move {
                    todo_repository.find_by_id(&db_context, todo_id).await
                })
            })
            .await;

        result.map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
    }

    pub async fn update_todo(
        &self,
        mut todo: Todo,
        new_description: String,
    ) -> Result<Todo, Box<dyn std::error::Error>> {
        let todo_repository = self.todo_repository.clone();

        let result = self
            .transaction_manager
            .transaction(
                |db_context| -> Pin<Box<dyn Future<Output = Result<Todo, TM::Error>> + Send>> {
                    let todo_repository = todo_repository.clone();
                    Box::pin(async move {
                        todo.update_description(new_description);
                        todo_repository.update(&db_context, todo).await
                    })
                },
            )
            .await;

        result.map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
    }
}
