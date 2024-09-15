use rbatis::RBatis;
use rbatis::table_sync::SqliteTableMapper;
use serde::Serialize;
use tracing::{debug, instrument, trace};
use crate::types::StationTransfer;

#[instrument]
pub async fn init_db() -> anyhow::Result<RBatis> {
    trace!("Connecting to database");
    let rb = new_db_connection()?;
    
    trace!("Setting up tables");
    sync_table::<StationTransfer>(&rb, "station_transfer").await?;

    Ok(rb)
}

pub async fn add_indices(rb: &RBatis) -> anyhow::Result<Vec<String>> {
    let mut names = Vec::new();
    debug!("Adding indices, this may take a while...");

    // Add indices
    names.push(add_index(rb, "station_transfer", &["station_code"]).await?);
    
    Ok(names)
}

pub async fn drop_indices(rb: &RBatis, names: &[String]) -> anyhow::Result<()> {
    debug!("Dropping indices, this may take a while...");
    for name in names {
        drop_index(rb, name).await?
    }
    
    Ok(())
}

/// Get another database connection.
/// `init_db` should be used for the first connection in order to properly
/// set up the database.
pub fn new_db_connection() -> anyhow::Result<RBatis> {
    let rb = RBatis::new();
    rb.init(
        rbdc_sqlite::driver::SqliteDriver {},
        "sqlite://sqlite.db",
    )?;
    Ok(rb)
}

async fn sync_table<T>(rb: &RBatis, table_name: &str) -> anyhow::Result<()>
where
    T: Default + Serialize,
{
    RBatis::sync(
        &rb.acquire().await?,
        &SqliteTableMapper {},
        &T::default(),
        table_name,
    ).await?;
    Ok(())
}

async fn add_index(rb: &RBatis, table: &str, columns: &[&str]) -> anyhow::Result<String> {
    let name = columns.join("_") + "_idx_" + table;
    rb.query(
        format!("CREATE INDEX IF NOT EXISTS {name} ON {table} ({});",
                columns.join(", ")).as_str(),
        vec![],
    ).await?;

    Ok(name)
}

async fn drop_index(rb: &RBatis, name: &String) -> anyhow::Result<()> {
    rb.exec(
        format!("DROP INDEX IF EXISTS {name}").as_str(), vec![]
    ).await?;
    
    Ok(())
}
