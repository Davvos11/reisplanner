use rbatis::RBatis;
use tracing::{debug, instrument, trace};
use reisplanner_gtfs::gtfs::types::{Agency, CalendarDate, FeedInfo, LastUpdated, Route, Shape, Stop, StopTime, Transfer, Trip};
use reisplanner_utils::database::{add_index, new_db_connection, sync_table};

#[instrument]
pub async fn init_db() -> anyhow::Result<RBatis> {
    trace!("Connecting to database");
    let rb = new_db_connection()?;
    
    trace!("Setting up tables");
    sync_table::<Agency>(&rb, "agency").await?;
    sync_table::<CalendarDate>(&rb, "calendar_date").await?;
    sync_table::<FeedInfo>(&rb, "feed_info").await?;
    sync_table::<Route>(&rb, "route").await?;
    sync_table::<Shape>(&rb, "shape").await?;
    sync_table::<Stop>(&rb, "stop").await?;
    sync_table::<StopTime>(&rb, "stop_time").await?;
    sync_table::<Transfer>(&rb, "transfer").await?;
    sync_table::<Trip>(&rb, "trip").await?;
    sync_table::<LastUpdated>(&rb, "last_updated").await?;

    Ok(rb)
}

pub async fn add_indices(rb: &RBatis) -> anyhow::Result<Vec<String>> {
    let mut names = Vec::new();
    debug!("Adding indices, this may take a while...");

    // Add indices
    names.push(add_index(rb, "trip", &["trip_id"]).await?);
    names.push(add_index(rb, "trip", &["trip_id", "trip_long_name"]).await?);
    names.push(add_index(rb, "stop_time", &["stop_id", "trip_id"]).await?);
    names.push(add_index(rb, "stop_time", &["stop_sequence", "trip_id"]).await?);
    names.push(add_index(rb, "stop_time", &["trip_id"]).await?);
    names.push(add_index(rb, "stop_time", &["id"]).await?);
    names.push(add_index(rb, "route", &["route_id"]).await?);
    names.push(add_index(rb, "stop", &["stop_id"]).await?);

    Ok(names)
}

