use std::time::SystemTime;
use indicatif::ProgressBar;
use rbatis::executor::Executor;
use reisplanner_gtfs::gtfs::types::{Route, Stop, StopTime, Trip};
use crate::database::new_db_connection;

#[cfg(test)]
mod tests;
mod database;


#[derive(Debug)]
struct Connection {
    departure_station: u32,
    arrival_station: u32,
    departure_timestamp: u32,
    arrival_timestamp: u32,
    route_description: String,
}

impl Connection {
    pub async fn arrival_name(&self, db: &impl Executor) -> anyhow::Result<String> {
        let name =
            Stop::select_by_id(db, &self.arrival_station).await?
                .ok_or(anyhow::Error::msg("Stop not found"))?
                .stop_name;
        Ok(name)
    }
    pub async fn departure_name(&self, db: &impl Executor) -> anyhow::Result<String> {
        let name =
            Stop::select_by_id(db, &self.departure_station).await?
                .ok_or(anyhow::Error::msg("Stop not found"))?
                .stop_name;
        Ok(name)
    }
}

async fn generate_timetable() -> anyhow::Result<()> {
    let db = new_db_connection()?;
    eprintln!("Getting trips...");
    let trips = Trip::select_all(&db).await?;
    let mut timetable = Vec::new();

    eprintln!("Generating timetable...");
    let bar = ProgressBar::new(trips.len() as u64);

    for trip in &trips {
        let route = Route::select_by_id(&db, &trip.route_id).await?.unwrap();
        let stops = StopTime::select_by_trip_id(&db, &trip.trip_id).await?;
        for window in stops.windows(2) {
            if let [dep_stop, arr_stop] = window {
                timetable.push(Connection {
                    departure_station: dep_stop.stop_id,
                    arrival_station: arr_stop.stop_id,
                    departure_timestamp: dep_stop.departure_time.into(),
                    arrival_timestamp: arr_stop.arrival_time.into(),
                    route_description: format!("{} {} {}", route.agency_id, route.route_short_name, route.route_long_name),
                });
            }
        }

        bar.inc(1);
    }

    bar.finish();

    Ok(())
}