use rbatis::executor::Executor;
use reisplanner_gtfs::gtfs::types::Stop;

pub async fn get_stop(stop_id: u32, db: &impl Executor) -> anyhow::Result<Stop> {
    let stop = Stop::select_by_id(db, &stop_id).await?
        .ok_or(anyhow::Error::msg("Stop not found"))?;
    Ok(stop)
}

pub async fn get_stop_str(stop_id: &String, db: &impl Executor) -> anyhow::Result<Stop> {
    let stop = Stop::select_by_id_str(db, stop_id).await?
        .ok_or(anyhow::Error::msg("Stop not found"))?;
    Ok(stop)
}

pub async fn get_parent_station(stop_id: u32, db: &impl Executor) -> anyhow::Result<Stop> {
    let stop = get_stop(stop_id, db).await?;
    match stop.parent_station {
        None => {
            Ok(stop)
        }
        Some(parent_id) => {
            Ok(get_stop_str(&parent_id, db).await?)
        }
    }
}

