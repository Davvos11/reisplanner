use std::error::Error;
use rbatis::RBatis;
use rbatis::table_sync::SqliteTableMapper;
use crate::gtfs::types::Agency;

pub async fn init_db() -> Result<RBatis, Box<dyn Error>> {
    let rb = RBatis::new();
    rb.init(
        rbdc_sqlite::driver::SqliteDriver {},
        "sqlite://sqlite.db",
    )?;
    
    RBatis::sync(
        &rb.acquire().await?,
        &SqliteTableMapper {},
        &Agency::default(),
        "agency"
    ).await?;
    
    Ok(rb)
}