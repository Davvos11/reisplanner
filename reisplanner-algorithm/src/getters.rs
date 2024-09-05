use rbatis::executor::Executor;

use reisplanner_gtfs::gtfs::types::Stop;

pub async fn get_stop(stop_id: &u32, db: &impl Executor) -> anyhow::Result<Stop> {
    let stop = Stop::select_by_id(db, stop_id).await?;
    match stop {
        None => { get_stop_str(&format!("stoparea:{stop_id}"), db).await }
        Some(stop) => { Ok(stop) }
    }
}

pub async fn get_stop_str(stop_id: &String, db: &impl Executor) -> anyhow::Result<Stop> {
    let stop = Stop::select_by_id_str(db, stop_id).await?
        .ok_or(anyhow::Error::msg("Stop not found"))?;
    Ok(stop)
}

pub async fn get_stop_readable(stop_id: &u32, db: &impl Executor) -> anyhow::Result<String> {
    let stop = get_stop(stop_id, db).await?;
    let mut result = stop.stop_name;
    if let Some(platform) = stop.platform_code {
        result = format!("{result} {platform}");
    }
    Ok(result)
}