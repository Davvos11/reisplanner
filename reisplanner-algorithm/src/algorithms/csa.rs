use std::collections::HashMap;

use indicatif::ProgressBar;
use rbatis::executor::Executor;
use serde::{Deserialize, Serialize};
use tokio::time::Instant;

use reisplanner_gtfs::gtfs::types::{Route, StopTime, Trip};

use crate::benchmark;
use crate::getters::{get_parent_station, get_stop_str};
use crate::utils::{deserialize_from_disk, seconds_to_hms, serialize_to_disk};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Connection {
    departure_station: String,
    arrival_station: String,
    departure_timestamp: u32,
    arrival_timestamp: u32,
    route_description: String,
}

impl Connection {
    pub async fn arrival_name(&self, db: &impl Executor) -> anyhow::Result<String> {
        Ok(get_stop_str(&self.arrival_station, db).await?.stop_name)
    }
    pub async fn departure_name(&self, db: &impl Executor) -> anyhow::Result<String> {
        Ok(get_stop_str(&self.departure_station, db).await?.stop_name)
    }
}

async fn generate_timetable(db: &impl Executor) -> anyhow::Result<Vec<Connection>> {
    eprintln!("Getting trips...");
    // let trips = Trip::select_all(db).await?;
    let trips = Trip::select_by_column(db, "trip_long_name", "Intercity").await?;
    let mut timetable = Vec::new();

    eprintln!("Generating timetable...");
    let bar = ProgressBar::new(trips.len() as u64);

    // let mut t = Instant::now();

    for trip in trips {
        let route = Route::select_by_id(db, &trip.route_id).await?.unwrap();
        // benchmark!(&mut t, "Get route");
        let stops = StopTime::select_by_trip_id(db, &trip.trip_id).await?;
        // benchmark!(&mut t, "Get stops");
        for window in stops.windows(2) {
            if let [dep_stop, arr_stop] = window {
                let dep_station = get_parent_station(dep_stop.stop_id, db).await?;
                // benchmark!(&mut t, "Get departure station");
                let arr_station = get_parent_station(arr_stop.stop_id, db).await?;
                // benchmark!(&mut t, "Get arrival station");
                timetable.push(Connection {
                    departure_station: dep_station.stop_id,
                    arrival_station: arr_station.stop_id,
                    departure_timestamp: dep_stop.departure_time.into(),
                    arrival_timestamp: arr_stop.arrival_time.into(),
                    route_description: format!("{} {} {}", route.agency_id, route.route_short_name, route.route_long_name),
                });
                // benchmark!(&mut t, "Create connection");
            }
        }

        bar.inc(1);
    }

    bar.finish();

    Ok(timetable)
}


fn csa_main_loop(timetable: &[Connection], arrival_station: String, earliest_arrival: &mut HashMap<String, u32>, in_connection: &mut HashMap<String, usize>) {
    let mut earliest = u32::MAX;

    for (i, connection) in timetable.iter().enumerate() {
        if connection.departure_timestamp >= *earliest_arrival.get(&connection.departure_station).unwrap_or(&u32::MAX) &&
            connection.arrival_timestamp < *earliest_arrival.get(&connection.arrival_station).unwrap_or(&u32::MAX) {
            earliest_arrival.insert(connection.arrival_station.clone(), connection.arrival_timestamp);
            in_connection.insert(connection.arrival_station.clone(), i);

            if connection.arrival_station == arrival_station &&
                connection.arrival_timestamp < earliest {
                earliest = connection.arrival_timestamp;
            }
        }
        // Don't break when later, our timetable is not sorted
        //     else if connection.arrival_timestamp > earliest {
        //         break;
        //     }
    }
}

const TIMETABLE: &str = "timetable.blob";

pub async fn get_timetable(db: &impl Executor, cache: bool) -> anyhow::Result<Vec<Connection>> {
    if !cache {
        generate_timetable(db).await
    } else {
        // Get timetable from disk or generate
        match deserialize_from_disk(TIMETABLE) {
            Ok(timetable) => { Ok(timetable) }
            Err(_) => {
                let timetable = generate_timetable(db).await?;
                serialize_to_disk(&timetable, TIMETABLE)?;
                Ok(timetable)
            }
        }
    }
}

pub async fn run_csa(
    departure: &String,
    arrival: &String,
    departure_time: impl Into<u32>,
    timetable: &[Connection],
) -> anyhow::Result<Option<Vec<Connection>>>
{
    let mut in_connection = HashMap::with_capacity(1000);
    let mut earliest_arrival = HashMap::with_capacity(1000);

    earliest_arrival.insert(departure.clone(), departure_time.into());

    let mut t = Instant::now();
    csa_main_loop(timetable, arrival.clone(), &mut earliest_arrival, &mut in_connection);
    benchmark!(&mut t, "Found route");

    if !in_connection.contains_key(arrival) {
        Ok(None)
    } else {
        let mut route = Vec::new();
        let mut last_connection_idx = in_connection.get(arrival);

        while let Some(index) = last_connection_idx {
            let connection = &timetable[*index];
            route.push(connection);
            last_connection_idx = in_connection.get(&connection.departure_station);
        }
        
        let route = route.into_iter().cloned().rev().collect();
        Ok(Some(route))
    }
}

pub async fn print_result(result: &[Connection], db: &impl Executor) -> anyhow::Result<()> {
    for connection in result {
        println!("{} @ {} - {} @ {} using {}",
                 connection.departure_name(db).await?,
                 seconds_to_hms(connection.departure_timestamp),
                 connection.arrival_name(db).await?,
                 seconds_to_hms(connection.arrival_timestamp),
                 connection.route_description
        )
    }

    Ok(())
}