extern crate protobuf;

use crate::database::init_db;
use crate::gtfs::run_gtfs;
use crate::gtfs_realtime_parse::run_gtfs_realtime;

pub mod gtfs_realtime;
pub mod gtfs;
mod gtfs_realtime_parse;
pub mod utils;
pub mod database;
pub mod rbatis_wrapper;
mod errors;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // TODO use tracing instead of println
    let db = init_db().await?;
    // Run initial GTFS download and database update (if needed)
    run_gtfs(&db, false).await?;
    run_gtfs_realtime(&db).await?;
    Ok(())
}
