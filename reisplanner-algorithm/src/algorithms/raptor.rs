use std::cmp::{min, Ordering};
use std::collections::{BTreeSet, HashMap, HashSet};
use std::hash::Hash;
use std::process::exit;
use indicatif::{ProgressBar, ProgressStyle};
use rbatis::executor::Executor;
use rbatis::RBatis;
use serde::{Deserialize, Serialize};
use tracing::debug;
use tracing::field::debug;
use reisplanner_gtfs::gtfs::types::{Route, Trip};
use crate::database::queries::{count_stop_times, get_parent_station_map, get_stop_times, get_trip_route_map};
use crate::getters::get_stop;
use crate::utils::{deserialize_from_disk, seconds_to_hms, serialize_to_disk};

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
pub struct Connection {
    /// Parent stop_id but without the `stopearea:` prefix
    departure_station: u32,
    /// Parent stop_id but without the `stopearea:` prefix
    arrival_station: u32,
    departure: u32,
    arrival: u32,
    trip_id: u32,
}

impl Connection {
    pub async fn arrival_name(&self, db: &impl Executor) -> anyhow::Result<String> {
        Ok(get_stop(&self.arrival_station, db).await?.stop_name)
    }
    pub async fn departure_name(&self, db: &impl Executor) -> anyhow::Result<String> {
        Ok(get_stop(&self.departure_station, db).await?.stop_name)
    }
    pub async fn route_information(&self, db: &impl Executor) -> anyhow::Result<String> {
        let trip = Trip::select_by_id(db, &self.trip_id).await?
            .ok_or(anyhow::Error::msg("Cannot get trip"))?;
        let route = Route::select_by_id(db, &trip.route_id).await?
            .ok_or(anyhow::Error::msg("Cannot get route"))?;
        Ok(format!("{} {} {}", route.agency_id, route.route_short_name, route.route_long_name))
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
/// Note: this is not the same concept as a GTFS route, this
/// is the same as a GTFS trip
pub struct RRoute {
    stops: Vec<u32>,
    // TODO this is always just one vec because the stop_times are ordered
    //  Also, it seems that in the OVapi GTFS all trips just contain
    //  a single set of stop_times and not multiple sets
    connections: Vec<Vec<Connection>>, 
}

impl RRoute {
    pub fn new(stops: Vec<u32>) -> Self {
        Self {
            stops,
            connections: Vec::new(),
        }
    }

    pub fn is_before(&self, p1: &u32, p2: &u32) -> bool {
        for stop in &self.stops {
            if stop == p1 {
                return true;
            } else if stop == p2 {
                return false;
            }
        }

        panic!("Stops {p1} and {p2} not in this route")
    }
    
    pub fn stops_from(&self, p: &u32) -> Vec<(usize, u32)> {
        // TODO try not to clone
        for (i, stop) in self.stops.iter().enumerate() {
            if stop == p {
                return self.stops[i..].iter().cloned()
                    .zip(i..)
                    .map(|(item, i)| (i, item))
                    .collect()
            }
        }
        
        panic!("Stop {p} not in this route")
    }

    pub fn trip_from(&self, stop_index: &u32, start_time: &u32) -> Option<&Vec<Connection>> {
        let mut left: usize = 0;
        let mut right: usize = self.connections.len() - 1;
        let mut ans = None;
        while left <= right {
            let mid = (left + right) / 2;

            if &self.connections[mid][*stop_index as usize].departure < start_time {
                left = mid + 1;
            } else {
                ans = Some(&self.connections[mid]);

                if mid == 0 {
                    break;
                }

                right = mid - 1;
            }

        };

        ans
    }
}

const PAGE_SIZE: u64 = 1_000_000;

async fn generate_timetable(db: &RBatis) -> anyhow::Result<HashMap<u32, RRoute>> {
    debug!("Getting trips, routes and stations...");
    let parent_stations = get_parent_station_map(db).await?;
    let total_count = count_stop_times(db).await?;

    debug("Generating timetable from stop_times, this will take a while...");
    let mut routes = HashMap::new(); // trip_id -> RRoute
    let bar = ProgressBar::new(total_count);
    bar.set_style(ProgressStyle::default_bar()
        .template("{wide_bar} Elapsed: {elapsed_precise}, ETA: {eta_precise}")?);

    let mut highest_id = 0;
    loop {
        let stop_times = get_stop_times(highest_id, PAGE_SIZE, db).await?;
        let count = stop_times.len();

        let stop_connections = stop_times.windows(2);

        let mut need_to_get_stops = true;
        let mut current_trip_stops = Vec::new();
        let mut current_trip_connections = Vec::new();

        for window in stop_connections {
            if let [dep_stop, arr_stop] = window {
                let dep_station = parent_stations.get(&dep_stop.stop_id)
                    .ok_or(anyhow::Error::msg("Parent station not found"))?;
                let arr_station = parent_stations.get(&arr_stop.stop_id)
                    .ok_or(anyhow::Error::msg("Parent station not found"))?;

                if need_to_get_stops {
                    current_trip_stops.push(*dep_station);
                }
                let connection = Connection{
                    departure_station: *dep_station,
                    arrival_station: *arr_station,
                    departure: dep_stop.departure_time.into(),
                    arrival: arr_stop.arrival_time.into(),
                    trip_id: dep_stop.trip_id,
                };
                current_trip_connections.push(connection);

                // Reset temporary storages and write values to result
                // This if statement does not trigger at the very last iteration
                // That's why we also write this at count != PAGE_SIZE below.
                if dep_stop.trip_id != arr_stop.trip_id {
                    let route = routes.entry(dep_stop.trip_id)
                        .or_insert(RRoute::new(current_trip_stops));
                    route.connections.push(current_trip_connections);

                    current_trip_stops = Vec::new();
                    current_trip_connections = Vec::new();
                    // We only need to get stops (for the next trip) if we haven't already
                    need_to_get_stops = !routes.contains_key(&arr_stop.trip_id);
                }
            }
        }

        bar.inc(count as u64);
        if count != PAGE_SIZE as usize {
            if let Some(last_stop) = stop_times.last() {
                let route = routes.entry(last_stop.trip_id)
                    .or_insert(RRoute::new(current_trip_stops));
                route.connections.push(current_trip_connections);
            }
            break;
        } else {
            highest_id = stop_times.last().unwrap().id.unwrap();
        }
    }
    bar.finish();

    Ok(routes)
}


const TIMETABLE: &str = "raptor_timetable.blob";

// TODO make generic
pub async fn get_timetable(db: &RBatis, cache: bool) -> anyhow::Result<HashMap<u32, RRoute>> {
    if !cache {
        generate_timetable(db).await
    } else {
        // Get timetable from disk or generate
        match deserialize_from_disk(TIMETABLE) {
            Ok(timetable) => Ok(timetable),
            Err(_) => {
                let timetable = generate_timetable(db).await?;
                serialize_to_disk(&timetable, TIMETABLE)?;
                Ok(timetable)
            }
        }
    }
}

const MAX_K: usize = 5;
const MAX_STATIONS: usize = 1000000;

pub async fn run_raptor(
    departure: u32,
    arrival: u32,
    departure_time: impl Into<u32>,
    timetable: &HashMap<u32, RRoute>,
) -> anyhow::Result<Option<Vec<(&Connection, &Connection)>>> {
    let departure_time = departure_time.into();
    let mut earliest_k_arrival: Vec<Vec<u32>> =
        vec![vec![u32::MAX - 3600 * 4; MAX_K + 1]; MAX_STATIONS];
    earliest_k_arrival[departure as usize][0] = departure_time;

    let mut earliest_arrival: Vec<u32> = vec![u32::MAX - 3600 * 4; MAX_STATIONS];
    earliest_arrival[departure as usize] = departure_time;

    let mut interchange: Vec<Option<(usize, usize, u32)>> =
        vec![None; MAX_STATIONS];
    let mut prev: Vec<Option<(&Connection, &Connection, (usize, usize, u32))>> =
        vec![None; MAX_STATIONS];

    let mut marked = HashSet::new();
    marked.insert(departure);

    // Main loop:
    for k in 1..=MAX_K {
        // Accumulate routes serving marked stops from previous round
        let mut q: HashMap<u32, u32> = HashMap::new();

        for p in &marked {
            // For each route r serving p:
            for (r, route) in timetable {
                if !route.stops.contains(p) {
                    continue;
                }
                if let Some(p2) = q.get(r) {
                    if !route.is_before(p, p2) {
                        continue;
                    }
                }
                // If (r, p') in Q: replace it with (r, p) if p comes before p'
                // If (r, p') not in Q, add (r, p) to Q
                q.insert(*r, *p);
            }
        }
        // dbg!(&marked);
        // dbg!(&q);
        marked.clear();

        println!("{k} --------------------------------");
        // Traverse each route in Q
        for (r, p) in q.iter() {
            // t = the current trip
            let mut t: Option<&Vec<Connection>> = None;
            let mut t_from = 0;

            let route = timetable.get(r)
                .ok_or(anyhow::Error::msg("Route not found"))?;
            // dbg!(&route.stops);
            // dbg!(r, p, route);
            // For each stop p_i of r beginning with p
            for (i, p_i) in route.stops_from(p) {
                // dbg!(i, p_i);
                let p_i = p_i as usize;
                // Can the label be improved in this round?
                if let Some(t) = t {
                    if t[i-1].arrival < min(earliest_arrival[arrival as usize], earliest_arrival[p_i]) {
                        earliest_k_arrival[p_i][k] = t[i-1].arrival;
                        earliest_arrival[p_i] = t[i-1].arrival;
                        prev[p_i] = Some((&t[t_from], &t[i-1], interchange[t[t_from].departure_station as usize].unwrap()));
                        marked.insert(p_i as u32);
                    }
                }
                // Can we catch an earlier trip at p_i
                if t.is_none() || earliest_k_arrival[p_i][k-1] < t.unwrap()[i].departure {
                    // dbg!(earliest_k_arrival[p_i][k-1], &t.map(|t| &t[i]));
                    t = route.trip_from(&(i as u32), &earliest_k_arrival[p_i][k-1]);
                    // dbg!(i, earliest_k_arrival[p_i][k-1]);
                    interchange[p_i] = Some((p_i, p_i, 0)); // TODO
                    t_from = i;
                }
            }
        }
        // exit(0);

        // Look at footpaths

        if marked.is_empty() {
            break;
        }
    }
    
    let mut parts: Vec<_> = Vec::new();
    let mut cur = arrival as usize;
    while let Some(tuple@(c1, c2, (p1, p2, dur))) = prev[cur] {
        parts.push((c1, c2));
        // dbg!(&tuple);
        // parts.push(TripPart::Connection(c1, c2));
        // parts.push(TripPart::Footpath(p1, p2, dur));
        cur = c1.departure_station as usize;
    }
    
    parts.reverse();

    if parts.is_empty(){
        return Ok(None);
    }

    Ok(Some(parts))
}
pub async fn print_result(result: &[(&Connection, &Connection)], db: &impl Executor) -> anyhow::Result<()> {
    for &(connection_a, connection_b) in result {
        println!(
            "{} @ {} - {} @ {} using {}",
            connection_a.departure_name(db).await?,
            seconds_to_hms(connection_a.departure),
            connection_b.arrival_name(db).await?,
            seconds_to_hms(connection_b.arrival),
            connection_b.route_information(db).await?
        );
    }

    Ok(())
}

