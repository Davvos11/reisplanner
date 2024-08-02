use std::collections::HashMap;
use rbatis::executor::Executor;
use reisplanner_gtfs::gtfs::types::{Route, Stop, StopTime, Trip};

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


pub fn get_stop_map(stop_id: &String, stops: &HashMap<String, Stop>) -> anyhow::Result<Stop> {
    let stop = stops.get(stop_id)
        .ok_or(anyhow::Error::msg("Stop not found"))?;
    Ok(stop.clone())
}

pub fn get_parent_station_map(stop_id: u32, stops: &HashMap<String, Stop>) -> anyhow::Result<Stop> {
    let stop = get_stop_map(&stop_id.to_string(), stops)?;
    match &stop.parent_station {
        None => {
            Ok(stop)
        }
        Some(parent_id) => {
            Ok(get_stop_map(parent_id, stops)?)
        }
    }
}

pub fn get_route<'a>(trip_id: &u32, trips: &HashMap<u32, Trip>, routes: &'a HashMap<u32, Route>)
                 -> anyhow::Result<&'a Route> {
    let trip = trips.get(trip_id)
        .ok_or(anyhow::Error::msg("Trip not found"))?;
    let route = routes.get(&trip.route_id)
        .ok_or(anyhow::Error::msg("Route not found"))?;
    Ok(route)
}
