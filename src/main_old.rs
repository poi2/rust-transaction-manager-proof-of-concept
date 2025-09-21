use sqlx::{Acquire, PgConnection, Postgres, query};

async fn insert_and_verify(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    test_id: uuid::Uuid,
) -> Result<(), Box<dyn std::error::Error>> {
    query!(
        r#"INSERT INTO todo (id, description)
        VALUES ( $1, $2 )
        "#,
        test_id,
        "test todo"
    )
    // In 0.7, `Transaction` can no longer implement `Executor` directly,
    // so it must be dereferenced to the internal connection type.
    .execute(&mut **transaction)
    .await?;

    // check that inserted todo can be fetched inside the uncommitted transaction
    let _ = query!(r#"SELECT FROM todo WHERE id = $1"#, test_id)
        .fetch_one(&mut **transaction)
        .await?;

    Ok(())
}

async fn explicit_rollback_example(
    pool: &sqlx::PgPool,
    test_id: uuid::Uuid,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut transaction = pool.begin().await?;

    insert_and_verify(&mut transaction, test_id).await?;

    transaction.rollback().await?;

    Ok(())
}

async fn implicit_rollback_example(
    pool: &sqlx::PgPool,
    test_id: uuid::Uuid,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut transaction = pool.begin().await?;

    insert_and_verify(&mut transaction, test_id).await?;

    // no explicit rollback here but the transaction object is dropped at the end of the scope
    Ok(())
}

async fn commit_example(
    pool: &sqlx::PgPool,
    test_id: uuid::Uuid,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut transaction = pool.begin().await?;

    insert_and_verify(&mut transaction, test_id).await?;

    transaction.commit().await?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let conn_str =
        std::env::var("DATABASE_URL").expect("Env var DATABASE_URL is required for this example.");
    let pool = sqlx::PgPool::connect(&conn_str).await?;

    // let test_id = uuid::Uuid::new_v4();

    // // remove any old values that might be in the table already with this id from a previous run
    // let _ = query!(r#"DELETE FROM todo WHERE id = $1"#, test_id)
    //     .execute(&pool)
    //     .await?;

    // explicit_rollback_example(&pool, test_id).await?;

    // // check that inserted todo is not visible outside the transaction after explicit rollback
    // let inserted_todo = query!(r#"SELECT FROM todo WHERE id = $1"#, test_id)
    //     .fetch_one(&pool)
    //     .await;

    // assert!(inserted_todo.is_err());

    // implicit_rollback_example(&pool, test_id).await?;

    // // check that inserted todo is not visible outside the transaction after implicit rollback
    // let inserted_todo = query!(r#"SELECT FROM todo WHERE id = $1"#, test_id)
    //     .fetch_one(&pool)
    //     .await;

    // assert!(inserted_todo.is_err());

    // commit_example(&pool, test_id).await?;

    // // check that inserted todo is visible outside the transaction after commit
    // let inserted_todo = query!(r#"SELECT * FROM todo WHERE id = $1"#, test_id)
    //     .fetch_one(&pool)
    //     .await;

    // assert!(inserted_todo.is_ok());

    let todo_use_case = &mut TodoUseCase::new(pool.clone());
    let repo = &TodoRepository {};
    let todo = Todo {
        id: uuid::Uuid::new_v4(),
        description: "UseCase Test 1".to_string(),
    };
    let created = todo_use_case
        .create_todo_tx_pattern(repo, todo.clone())
        .await?;
    println!("created via tx pattern: {:?}", created);

    let todo = Todo {
        id: uuid::Uuid::new_v4(),
        description: "UseCase Test 2".to_string(),
    };
    let created = todo_use_case
        .create_todo_pool_pattern(repo, todo.clone())
        .await?;
    println!("created via pool pattern: {:?}", created);

    Ok(())
}

// Domain: Aggregate
#[allow(dead_code)]
#[derive(Debug, Clone)]
struct Todo {
    id: uuid::Uuid,
    description: String,
}

// Domain: Repository
#[allow(dead_code)]
trait ITodoRepository {
    // async fn create_todo<'a>(&self, executor: impl sqlx::PgExecutor<'a>, todo: Todo) -> Result<Todo, Box<dyn std::error::Error>>;
    async fn create_todo<'a>(&mut self, todo: Todo) -> Result<Todo, Box<dyn std::error::Error>>;
    // fn get_todo(&self, id: uuid::Uuid) -> Option<Todo>;
}

// Domain: Repository type
pub trait PgAcquire<'c>: Acquire<'c, Database = Postgres> + Send {}

impl<'c, T> PgAcquire<'c> for T where T: Acquire<'c, Database = Postgres> + Send {}

// Infrastructure: Repository Implementation
// #[allow(dead_code)]
// struct TodoRepository {}

#[allow(dead_code)]
struct TodoRepository2<E> {
    executor: E,
}

impl<E> ITodoRepository for TodoRepository2<E>
where
    for<'c> E: PgAcquire<'c> + Send + Sync,
    E: Clone + Copy,
{
    async fn create_todo<'a>(&mut self, todo: Todo) -> Result<Todo, Box<dyn std::error::Error>> {
        let mut conn = self.executor.acquire().await?;

        query!(
            r#"INSERT INTO todo (id, description)
            VALUES ( $1, $2 )
            "#,
            todo.id.clone(),
            todo.description.clone(),
        )
        .execute(&mut *conn)
        .await?;

        // check that inserted todo can be fetched inside the uncommitted transaction
        let record = query!(r#"SELECT * FROM todo WHERE id = $1"#, todo.id.clone())
            .fetch_one(&mut *conn)
            .await?;

        let returning = Todo {
            id: record.id,
            description: record.description,
        };

        Ok(returning)
    }
}

// Application: UseCase
struct TodoUseCase {
    pg_pool: sqlx::PgPool,
}

// impl TodoUseCase {
//     pub fn new(pg_pool: sqlx::PgPool) -> Self {
//         Self { pg_pool }
//     }

//     async fn create_todo_tx_pattern<'a>(&mut self, repo: &impl ITodoRepository, todo: Todo) -> Result<Todo, Box<dyn std::error::Error>> {
//         let conn = &mut *self.pg_pool.acquire().await?;
//         let mut tx = conn.begin().await?;

//         let created = repo.create_todo(&mut tx, todo).await?;

//         tx.commit().await?;

//         Ok(created)
//     }

//     async fn create_todo_pool_pattern<'a>(&mut self, repo: &impl ITodoRepository, todo: Todo) -> Result<Todo, Box<dyn std::error::Error>> {
//         let created = repo.create_todo(&self.pg_pool, todo).await?;

//         Ok(created)
//     }
// }

async fn run_query_1<'a, A>(conn: A) -> Result<(), Box<dyn std::error::Error>>
where
    A: Acquire<'a, Database = Postgres>,
{
    let mut conn = conn.acquire().await?;

    sqlx::query!("SELECT 1 as v").fetch_one(&mut *conn).await?;
    sqlx::query!("SELECT 2 as v").fetch_one(&mut *conn).await?;

    Ok(())
}

fn run_query_2<'a, 'c, A>(
    conn: A,
) -> impl Future<Output = Result<(), Box<dyn std::error::Error>>> + Send + 'a
where
    A: Acquire<'c, Database = Postgres> + Send + 'a,
{
    async move {
        let mut conn = conn.acquire().await?;

        sqlx::query!("SELECT 1 as v").fetch_one(&mut *conn).await?;
        sqlx::query!("SELECT 2 as v").fetch_one(&mut *conn).await?;

        Ok(())
    }
}
