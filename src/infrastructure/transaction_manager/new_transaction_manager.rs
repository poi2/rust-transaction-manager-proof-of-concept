use std::sync::Arc;

use anyhow::Result as AnyhowResult;
use async_trait::async_trait;
use sqlx::{PgPool, Postgres, Transaction};
use tokio::sync::{Mutex, MutexGuard};

/// Transaction manager interface.
/// Manage transactions used by the application and provide for each use case.
///
/// # Parameters
/// - `T`: Transaction type.
/// - `C`: Connection type.
#[async_trait]
pub trait NewTransactionManager<T, C>: Send + Sync {
    /// Start a transaction.
    async fn begin(&self) -> AnyhowResult<()>;

    /// Returns whether or not a Transaction has been initiated.
    async fn is_transaction_started(&self) -> bool;

    /// Get current transaction.
    async fn get_transaction(&self) -> AnyhowResult<MutexGuard<'_, Option<T>>>;

    /// Get connection.
    async fn get_connection(&self) -> AnyhowResult<C>;

    /// Commit transaction.
    async fn commit(&self) -> AnyhowResult<()>;

    /// Roll back a transaction.
    async fn rollback(&self) -> AnyhowResult<()>;
}

pub struct PsqlTransactionManager<'a> {
    pool: Arc<PgPool>,
    transaction: Arc<Mutex<Option<Transaction<'a, Postgres>>>>,
}

impl PsqlTransactionManager<'_> {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self {
            pool,
            transaction: Arc::new(Mutex::new(None)),
        }
    }
}

#[async_trait]
impl<'a> NewTransactionManager<Transaction<'a, Postgres>, Arc<PgPool>>
    for PsqlTransactionManager<'a>
{
    async fn begin(&self) -> AnyhowResult<()> {
        let mut tx_lock = self.transaction.lock().await;
        if tx_lock.is_some() {
            return Err(anyhow::anyhow!("Transaction already started"));
        }
        let tx = self.pool.begin().await?;
        *tx_lock = Some(tx);
        Ok(())
    }

    async fn is_transaction_started(&self) -> bool {
        let tx_lock = self.transaction.lock().await;
        tx_lock.is_some()
    }

    async fn get_transaction(
        &self,
    ) -> AnyhowResult<MutexGuard<'_, Option<Transaction<'a, Postgres>>>> {
        let tx_lock = self.transaction.lock().await;
        if tx_lock.is_none() {
            return Err(anyhow::anyhow!("No active transaction"));
        }
        Ok(tx_lock)
    }

    async fn get_connection(&self) -> AnyhowResult<Arc<PgPool>> {
        Ok(self.pool.clone())
    }

    async fn commit(&self) -> AnyhowResult<()> {
        let mut tx_lock = self.transaction.lock().await;
        if let Some(tx) = tx_lock.take() {
            tx.commit().await?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("No active transaction to commit"))
        }
    }

    async fn rollback(&self) -> AnyhowResult<()> {
        let mut tx_lock = self.transaction.lock().await;
        if let Some(tx) = tx_lock.take() {
            tx.rollback().await?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("No active transaction to rollback"))
        }
    }
}

// // 新しいシンプル版 - new_ prefix付き
// #[async_trait]
// pub trait NewPsqlTransactionManager: Send + Sync {
//     async fn begin(&self) -> AnyhowResult<()>;
//     async fn commit(&self) -> AnyhowResult<()>;
//     async fn rollback(&self) -> AnyhowResult<()>;
//     async fn is_transaction_started(&self) -> bool;
//     async fn get_transaction(
//         &self,
//     ) -> AnyhowResult<MutexGuard<'_, Option<Transaction<'_, Postgres>>>>;
//     async fn get_connection(&self) -> AnyhowResult<PgPool>;
// }

// pub struct NewPsqlTransactionManagerImpl {
//     pool: PgPool,
//     transaction: Arc<Mutex<Option<Transaction<'static, Postgres>>>>,
// }

// impl NewPsqlTransactionManagerImpl {
//     pub fn new(pool: PgPool) -> Self {
//         Self {
//             pool,
//             transaction: Arc::new(Mutex::new(None)),
//         }
//     }
// }

// #[async_trait]
// impl NewPsqlTransactionManager for NewPsqlTransactionManagerImpl {
//     async fn begin(&self) -> AnyhowResult<()> {
//         let mut tx_lock = self.transaction.lock().await;
//         if tx_lock.is_some() {
//             return Err(anyhow::anyhow!("Transaction already started"));
//         }

//         let tx = self.pool.begin().await?;
//         // SAFETY: Transactionのlifetimeを'staticに変換
//         // このTransactionは明示的にcommit/rollbackされるまでこの構造体が保持する
//         let tx: Transaction<'static, Postgres> = unsafe { std::mem::transmute(tx) };

//         *tx_lock = Some(tx);
//         Ok(())
//     }

//     async fn commit(&self) -> AnyhowResult<()> {
//         let mut tx_lock = self.transaction.lock().await;
//         if let Some(tx) = tx_lock.take() {
//             tx.commit().await?;
//             Ok(())
//         } else {
//             Err(anyhow::anyhow!("No active transaction to commit"))
//         }
//     }

//     async fn rollback(&self) -> AnyhowResult<()> {
//         let mut tx_lock = self.transaction.lock().await;
//         if let Some(tx) = tx_lock.take() {
//             tx.rollback().await?;
//             Ok(())
//         } else {
//             Err(anyhow::anyhow!("No active transaction to rollback"))
//         }
//     }

//     async fn is_transaction_started(&self) -> bool {
//         let tx_lock = self.transaction.lock().await;
//         tx_lock.is_some()
//     }

//     async fn get_transaction(
//         &self,
//     ) -> AnyhowResult<MutexGuard<'_, Option<Transaction<'_, Postgres>>>> {
//         let tx_lock = self.transaction.lock().await;
//         if tx_lock.is_none() {
//             return Err(anyhow::anyhow!("No active transaction"));
//         }
//         Ok(tx_lock)
//     }

//     async fn get_connection(&self) -> AnyhowResult<PgPool> {
//         let tx_lock = self.transaction.lock().await;
//         if tx_lock.is_some() {
//             return Err(anyhow::anyhow!(
//                 "Cannot get connection while transaction is active"
//             ));
//         }
//         Ok(self.pool.clone())
//     }
// }

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::query;

    // #[sqlx::test]
    // async fn test_new_transaction_lifecycle(pool: PgPool) {
    //     let mut mgr = NewPsqlTransactionManagerImpl::new(pool);

    //     // 初期状態
    //     assert!(!mgr.is_transaction_started().await);

    //     // Begin
    //     mgr.begin().await.unwrap();
    //     assert!(mgr.is_transaction_started().await);

    //     // Transaction使用
    //     let mut tx_guard = mgr.get_transaction().await.unwrap();
    //     let tx = tx_guard.as_mut().unwrap();
    //     let result = query!("SELECT 1 as value").fetch_one(&mut **tx).await;
    //     assert!(result.is_ok());

    //     // Commit
    //     mgr.commit().await.unwrap();
    //     assert!(!mgr.is_transaction_started().await);
    // }

    // #[sqlx::test]
    // async fn test_new_connection_single_operation(pool: PgPool) {
    //     let mgr = NewPsqlTransactionManagerImpl::new(pool);

    //     // 単発Connection取得
    //     let pool = mgr.get_connection().await.unwrap();
    //     let result = query!("SELECT 1 as value").fetch_one(&pool).await;
    //     assert!(result.is_ok());
    // }

    // #[sqlx::test]
    // async fn test_new_transaction_rollback(pool: PgPool) {
    //     let mut mgr = NewPsqlTransactionManagerImpl::new(pool);

    //     mgr.begin().await.unwrap();
    //     assert!(mgr.is_transaction_started().await);

    //     mgr.rollback().await.unwrap();
    //     assert!(!mgr.is_transaction_started().await);
    // }
}
