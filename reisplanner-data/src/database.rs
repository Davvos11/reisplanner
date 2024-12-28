use rbatis::RBatis;
use tracing::{instrument, trace};
use reisplanner_utils::database::{new_db_connection, sync_table};
use crate::types::{PlaceTransfer, StationTransfer};

#[instrument]
pub async fn init_db() -> anyhow::Result<RBatis> {
    trace!("Connecting to database");
    let rb = new_db_connection()?;
    
    trace!("Setting up tables");
    sync_table::<StationTransfer>(&rb, "station_transfer").await?;
    sync_table::<PlaceTransfer>(&rb, "place_transfer").await?;

    Ok(rb)
}
