use rbatis::RBatis;
use rbatis::table_sync::SqliteTableMapper;
use serde::Serialize;

use crate::gtfs::types::{Agency, CalendarDate, FeedInfo, Route, Shape, Stop, StopTime, Transfer, Trip};

pub async fn init_db() -> anyhow::Result<RBatis> {
    let rb = RBatis::new();
    rb.init(
        rbdc_sqlite::driver::SqliteDriver {},
        "sqlite://sqlite.db",
    )?;

    sync_table::<Agency>(&rb, "agency").await?;
    sync_table::<CalendarDate>(&rb, "calendar_date").await?;
    sync_table::<FeedInfo>(&rb, "feed_info").await?;
    sync_table::<Route>(&rb, "route").await?;
    sync_table::<Shape>(&rb, "shape").await?;
    sync_table::<Stop>(&rb, "stop").await?;
    sync_table::<StopTime>(&rb, "stop_time").await?;
    sync_table::<Transfer>(&rb, "transfer").await?;
    sync_table::<Trip>(&rb, "trip").await?;

    // Add indices
    add_index(&rb, "trip", &["trip_id"]).await?;
    add_index(&rb, "stop_time", &["stop_id", "trip_id"]).await?;
    add_index(&rb, "stop_time", &["stop_sequence", "trip_id"]).await?;

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

async fn add_index(rb: &RBatis, table: &str, columns: &[&str]) -> anyhow::Result<()> {
    let name = columns.join("_") + "_idx";
    rb.query(
        format!("CREATE INDEX IF NOT EXISTS {name} ON {table} ({});",
                columns.join(", ")).as_str(),
        vec![],
    ).await?;

    Ok(())
}
