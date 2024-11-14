use crate::database::queries::{count_stop_times, get_parent_station_map, get_stop_times, get_transfer_times};
use crate::getters::get_stop_readable;
use crate::types::JourneyPart;
use crate::utils::{deserialize_from_disk, serialize_to_disk};
use indicatif::{ProgressBar, ProgressStyle};
use rbatis::executor::Executor;
use rbatis::RBatis;
use reisplanner_gtfs::gtfs::types::{Route, Trip};
use serde::{Deserialize, Serialize};
use std::cmp::min;
use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use anyhow::anyhow;
use tracing::field::debug;
use tracing::{debug, trace};

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq, Copy, Hash)]
pub struct Location {
    stop_id: u32,
    /// Parent stop_id but without the `stopearea:` prefix
    parent_id: u32,
}


#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq, Copy)]
pub struct Arrival {
    time: u32,
    stop_id: u32,
}

impl Default for Arrival {
    fn default() -> Self {
        Self { time: u32::MAX - 3600, stop_id: 0 }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
pub struct Connection {
    pub departure_station: Location,
    pub arrival_station: Location,
    pub departure: u32,
    pub arrival: u32,
    pub trip_id: u32,
}

impl Connection {
    pub async fn arrival_name(&self, db: &impl Executor) -> anyhow::Result<String> {
        get_stop_readable(&self.arrival_station.stop_id, db).await
    }
    pub async fn departure_name(&self, db: &impl Executor) -> anyhow::Result<String> {
        get_stop_readable(&self.departure_station.stop_id, db).await
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
/// Note: while this is the same concept as a GTFS route, they are not equal.
/// In the OvApi GTFS set, most routes only have one trip.
/// In order to efficiently use Raptor, trips with the same stops should be
/// combined in the same routes.
pub struct RRoute {
    stops: Vec<Location>,
    connections: Vec<Vec<Connection>>,
}

impl RRoute {
    pub fn new(stops: Vec<Location>) -> Self {
        Self {
            stops,
            connections: Vec::new(),
        }
    }

    pub fn is_before(&self, p1: &u32, p2: &u32) -> bool {
        for stop in &self.stops {
            if stop.parent_id == *p1 {
                return true;
            } else if stop.parent_id == *p2 {
                return false;
            }
        }

        panic!("Stops {p1} and {p2} not in this route")
    }

    pub fn stops_from(&self, p: &u32) -> Vec<(usize, Location)> {
        // TODO try not to clone
        for (i, stop) in self.stops.iter().enumerate() {
            if stop.parent_id == *p {
                return self.stops[i..].iter().cloned()
                    .zip(i..)
                    .map(|(item, i)| (i, item))
                    .collect();
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

    pub fn contains_station(&self, stop_id: &u32) -> bool {
        self.stops.iter().map(|l| l.parent_id)
            .any(|s| s == *stop_id)
    }
}

const PAGE_SIZE: u64 = 1_000_000;

async fn generate_timetable(db: &RBatis) -> anyhow::Result<HashMap<u32, RRoute>> {
    debug!("Getting trips, routes and stations...");
    let parent_stations = get_parent_station_map(db).await?;
    let total_count = count_stop_times(db).await?;

    debug("Generating timetable from stop_times, this will take a while...");
    let mut routes = HashMap::new(); // [stop_id] -> RRoute
    let bar = ProgressBar::new(total_count);
    bar.set_style(ProgressStyle::default_bar()
        .template("{wide_bar} Elapsed: {elapsed_precise}, ETA: {eta_precise}")?);

    let mut highest_id = 0;
    loop {
        let stop_times = get_stop_times(highest_id, PAGE_SIZE, db).await?;
        let count = stop_times.len();

        let stop_connections = stop_times.windows(2);

        let mut current_trip_stops = Vec::new();
        let mut current_trip_connections = Vec::new();

        for window in stop_connections {
            if let [dep_stop, arr_stop] = window {
                let dep_parent_station = parent_stations.get(&dep_stop.stop_id)
                    .ok_or(anyhow::Error::msg("Parent station not found"))?;
                let arr_parent_station = parent_stations.get(&arr_stop.stop_id)
                    .ok_or(anyhow::Error::msg("Parent station not found"))?;

                // Use parent station for trip "identifier"
                current_trip_stops.push(Location { stop_id: dep_stop.stop_id, parent_id: *dep_parent_station });
                let connection = Connection {
                    departure_station: Location { stop_id: dep_stop.stop_id, parent_id: *dep_parent_station },
                    arrival_station: Location { stop_id: arr_stop.stop_id, parent_id: *arr_parent_station },
                    departure: dep_stop.departure_time.into(),
                    arrival: arr_stop.arrival_time.into(),
                    trip_id: dep_stop.trip_id,
                };
                current_trip_connections.push(connection);

                // Reset temporary storages and write values to result
                // This if statement does not trigger at the very last iteration
                // That's why we also write this at count != PAGE_SIZE below.
                if dep_stop.trip_id != arr_stop.trip_id {
                    // TODO clone
                    let route = routes.entry(current_trip_stops.clone())
                        .or_insert(RRoute::new(current_trip_stops));
                    route.connections.push(current_trip_connections);

                    current_trip_stops = Vec::new();
                    current_trip_connections = Vec::new();
                }
            }
        }

        bar.inc(count as u64);
        if count != PAGE_SIZE as usize {
            let route = routes.entry(current_trip_stops.clone())
                .or_insert(RRoute::new(current_trip_stops));
            route.connections.push(current_trip_connections);
            break;
        } else {
            highest_id = stop_times.last().unwrap().id.unwrap();
        }
    }
    bar.finish();

    // Change key type to be an integer
    let routes = routes.into_iter()
        .enumerate()
        .map(|(i, (_, v))| { (i as u32, v) })
        .collect();

    Ok(routes)
}

pub async fn generate_transfer_times(db: &RBatis) -> anyhow::Result<HashMap<u32, u32>> {
    let transfers = get_transfer_times(db).await?;
    Ok(transfers)
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

const MAX_K: usize = 10;

///
/// This code uses the following variable names that correspond with the
/// variables in Algorithm 1 in the Raptor paper:
/// Initialisation and lists:
///     departure_stop = p_s
///     arrival_stop = p_t
///     earliest_arrival = tau^*
///     earliest_k_arrival = tau^i
/// First step:
///     marked_stop = p
///     other_stop = p'
///     routes_marked_stops = Q
///     route (or route_id) = r
/// Second step:
///     marked_stop = p
///     stop_along_route = p_i
///     routes_marked_stops = Q
///     marked_route = r
///     trip = t
pub async fn run_raptor<'a>(
    departure_stop: u32,
    arrival_stop: u32,
    departure_time: impl Into<u32>,
    timetable: &'a HashMap<u32, RRoute>,
    transfers: &'a HashMap<u32, u32>,
) -> anyhow::Result<Option<Vec<JourneyPart>>> {
    let departure_time = departure_time.into();
    let mut earliest_k_arrival =
        vec![HashMap::new(); MAX_K + 1];
    earliest_k_arrival[0].insert(departure_stop, Arrival { time: departure_time, stop_id: 0 });

    let mut earliest_arrival = HashMap::new();
    earliest_arrival.insert(departure_stop, Arrival { time: departure_time, stop_id: 0 });

    let mut interchange: HashMap<u32, (u32, u32, u32)> = HashMap::new();
    let mut prev = HashMap::new();

    let mut marked = HashSet::new();
    marked.insert(departure_stop);

    let default_arrival = Arrival::default();

    // Main loop:
    let mut last_k = 0;
    for k in 1..=MAX_K {
        last_k = k;
        trace!("{k}th raptor loop...");
        // Accumulate routes serving marked stops from previous round
        let mut routes_marked_stops: HashMap<u32, u32> = HashMap::new();

        for marked_stop in &marked {
            // For each route r serving p:
            for (route_id, route) in timetable {
                if !route.contains_station(marked_stop) {
                    continue;
                }
                if let Some(other_stop) = routes_marked_stops.get(route_id) {
                    if !route.is_before(marked_stop, other_stop) {
                        continue;
                    }
                }
                // If (r, p') in Q: replace it with (r, p) if p comes before p'
                // If (r, p') not in Q, add (r, p) to Q
                routes_marked_stops.insert(*route_id, *marked_stop);
            }
        }
        marked.clear();

        // Traverse each route in Q
        for (marked_route, marked_stop) in routes_marked_stops.iter() {
            // t = the current trip
            let mut trip: Option<&Vec<Connection>> = None;
            let mut previous_stop_index = 0;

            let route = timetable.get(marked_route)
                .ok_or(anyhow::Error::msg("Route not found"))?;
            // For each stop p_i of r beginning with p
            for (stop_index, location_along_route) in route.stops_from(marked_stop) {
                let stop_along_route = location_along_route.stop_id;
                let parent_stop_along_route = location_along_route.parent_id;
                // Can the label be improved in this round?
                if let Some(trip) = trip {
                    let arrival_here = *earliest_arrival.get(&parent_stop_along_route).unwrap_or(&default_arrival);
                    let arrival_at_destination = *earliest_arrival.get(&arrival_stop).unwrap_or(&default_arrival);

                    if trip[stop_index - 1].arrival < min(arrival_here.time, arrival_at_destination.time) {
                        let arrival = Arrival { time: trip[stop_index - 1].arrival, stop_id: stop_along_route };
                        earliest_k_arrival[k].insert(parent_stop_along_route, arrival);
                        earliest_arrival.insert(parent_stop_along_route, arrival);
                        let previous_stop = &trip[previous_stop_index].departure_station.stop_id;
                        prev.insert(parent_stop_along_route,
                                    (&trip[previous_stop_index], &trip[stop_index - 1], *interchange.get(previous_stop).unwrap()));
                        marked.insert(parent_stop_along_route);
                    }
                }

                // Can we catch an earlier trip at p_i
                let transfer_time = if parent_stop_along_route == departure_stop { 0 } else {
                    60 * transfers.get(&stop_along_route)
                        .ok_or(anyhow::Error::msg("Transfer time not found"))?
                };
                let earliest_possible_transfer = earliest_k_arrival[k - 1].get(&parent_stop_along_route).unwrap_or(&default_arrival);
                let ept_time = earliest_possible_transfer.time + transfer_time;
                if trip.is_none() || ept_time < trip.unwrap()[stop_index].departure {
                    trip = route.trip_from(&(stop_index as u32), &ept_time);
                    interchange.insert(stop_along_route, (earliest_possible_transfer.stop_id, stop_along_route, transfer_time));
                    previous_stop_index = stop_index;
                }
            }
        }

        // Look at footpaths

        if marked.is_empty() {
            break;
        }
    }

    debug!("Finished Raptor path planning in {last_k} cycles");

    let mut parts: Vec<_> = Vec::new();
    let mut cur = arrival_stop;
    let mut seen = HashSet::new();
    while let Some(&(c1, c2, (p1, p2, dur))) = prev.get(&cur) {
        parts.push(JourneyPart::Vehicle(c1.clone(), c2.clone()));
        if dur > 0 {
            parts.push(JourneyPart::Transfer(p1, p2, dur));
        }
        cur = c1.departure_station.parent_id;

        let previous_size = seen.len();
        seen.insert(cur);
        if seen.len() == previous_size {
            let (stops, trips): (Vec<_>, Vec<_>) = parts.iter().filter_map(|p| {
                if let JourneyPart::Vehicle(c1, _) = p { Some((c1.departure_station.stop_id, c1.trip_id)) } else { None }
            }).collect();
            return Err(anyhow!("Loop in route, stops found so far (backwards): {stops:?} using trips {trips:?}"));
        }
    }

    parts.reverse();

    if parts.is_empty() {
        return Ok(None);
    }

    Ok(Some(parts))
}

pub async fn print_result(result: &Vec<JourneyPart>, db: &impl Executor) -> anyhow::Result<()> {
    for part in result {
        println!("{}", part.to_string(db).await?);
    }

    Ok(())
}

