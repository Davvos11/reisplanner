use async_trait::async_trait;
use rbatis::executor::Executor;
use serde::{Deserialize, Serialize};

#[async_trait]
pub trait DatabaseModel<T>
where T: for<'de> Deserialize<'de> + Serialize
{
    async fn insert_batch(
        executor: &dyn Executor,
        tables: &[T],
        batch_size: u64,
    ) -> Result<rbatis::rbdc::db::ExecResult, rbatis::rbdc::Error>;

    async fn delete_all(
        executor: &dyn Executor
    ) -> Result<rbatis::rbdc::db::ExecResult, rbatis::rbdc::Error>;

    async fn update_all(
        executor: &dyn Executor,
        table: &T,
    ) -> Result<rbatis::rbdc::db::ExecResult, rbatis::rbdc::Error>;
}


#[macro_export]
macro_rules! crud_trait {
    ($table:ty{}) => {
        rbatis::crud!($table {});
        rbatis::impl_delete!($table {delete_all() => "``"});
        rbatis::impl_update!($table {update_all() => "``"});

        #[async_trait]
        impl DatabaseModel<$table> for $table {
            async fn insert_batch(executor: &dyn Executor, tables: &[$table], batch_size: u64) -> Result<ExecResult, Error> {
                <$table>::insert_batch(executor, tables, batch_size).await
            }
            
            async fn delete_all(executor: &dyn Executor) -> Result<ExecResult, Error> {
                <$table>::delete_all(executor).await
            }
            
            async fn update_all(executor: &dyn Executor, table: &$table) -> Result<ExecResult, Error> {
                <$table>::update_all(executor, table).await
            }
        }
    };
}
