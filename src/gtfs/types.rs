use chrono::NaiveDate;
use serde::Deserialize;
use serde_repr::Deserialize_repr;
use crate::utils::{deserialize_date, deserialize_time_tuple, TimeTuple};

// Struct for agency.txt
#[derive(Debug, Deserialize)]
pub struct Agency {
    agency_id: String,
    agency_name: String,
    agency_url: String,
    agency_timezone: String,
    agency_phone: String,
}

#[derive(Deserialize_repr, PartialEq, Debug)]
#[repr(u8)]
enum ExceptionType {
    Added = 1,
    Removed = 2,
}

// Struct for calendar_dates.txt
#[derive(Debug, Deserialize)]
pub struct CalendarDate {
    service_id: u32,
    #[serde(deserialize_with = "deserialize_date")]
    date: NaiveDate,
    exception_type: ExceptionType,
}

// Struct for feed_info.txt
#[derive(Debug, Deserialize)]
pub struct FeedInfo {
    feed_publisher_name: String,
    feed_id: String,
    feed_publisher_url: String,
    feed_lang: String,
    #[serde(deserialize_with = "deserialize_date")]
    feed_start_date: NaiveDate,
    #[serde(deserialize_with = "deserialize_date")]
    feed_end_date: NaiveDate,
    feed_version: String,
}

#[derive(Deserialize_repr, PartialEq, Debug)]
#[repr(u8)]
enum RouteType {
    Tram = 0,
    Metro = 1,
    Train = 2,
    Bus = 3,
    Ferry = 4,
    CableTram = 5,
    Lift = 6,
    Funicular = 7,
    TrolleyBus = 11,
    MonoRail = 12,
}

// Struct for routes.txt
#[derive(Debug, Deserialize)]
pub struct Route {
    route_id: u32,
    agency_id: String,
    route_short_name: String,
    route_long_name: String,
    route_desc: Option<String>,
    route_type: RouteType,
    route_color: Option<String>,
    route_text_color: Option<String>,
    route_url: Option<String>,
}

// Struct for shapes.txt
#[derive(Debug, Deserialize)]
pub struct Shape {
    shape_id: u32,
    shape_pt_sequence: u32,
    shape_pt_lat: f64,
    shape_pt_lon: f64,
    shape_dist_traveled: Option<f64>,
}


#[derive(Deserialize_repr, PartialEq, Debug)]
#[repr(u8)]
enum LocationType {
    Stop = 0, // (Platform when defined within a parent_station)
    Station = 1,
    Entrance = 2,
    GenericNode = 3,
    BoardingArea = 4,
}


#[derive(Deserialize_repr, PartialEq, Debug)]
#[repr(u8)]
enum WheelchairBoarding {
    Empty = 0,
    SomePossible = 1,
    NotPossible = 2,
}

// Struct for stops.txt
#[derive(Debug, Deserialize)]
pub struct Stop {
    stop_id: u32,
    stop_code: Option<u32>,
    stop_name: String,
    stop_lat: f64,
    stop_lon: f64,
    location_type: LocationType,
    parent_station: Option<String>,
    stop_timezone: Option<String>,
    wheelchair_boarding: Option<WheelchairBoarding>,
    platform_code: Option<String>,
    zone_id: Option<String>,
}


#[derive(Deserialize_repr, PartialEq, Debug)]
#[repr(u8)]
enum PickupType {
    Regular = 0,
    NotAvailable = 1,
    MustPhone = 2,
    MustCoordinate = 3,
// pickup_type=0 forbidden if start_pickup_drop_off_window or end_pickup_drop_off_window are defined.
// pickup_type=3 forbidden if start_pickup_drop_off_window or end_pickup_drop_off_window are defined.
// drop_off_type=0 forbidden if start_pickup_drop_off_window or end_pickup_drop_off_window are defined.
}

// Struct for stop_times.txt
#[derive(Debug, Deserialize)]
pub struct StopTime {
    trip_id: u32,
    stop_sequence: u32,
    stop_id: u32,
    stop_headsign: Option<String>,
    #[serde(deserialize_with = "deserialize_time_tuple")]
    arrival_time: TimeTuple,
    #[serde(deserialize_with = "deserialize_time_tuple")]
    departure_time: TimeTuple,
    pickup_type: PickupType,
    drop_off_type: PickupType,
    timepoint: i32,
    shape_dist_traveled: Option<f64>,
    fare_units_traveled: Option<i32>,
}


#[derive(Deserialize_repr, PartialEq, Debug)]
#[repr(u8)]
enum TransferType {
    Recommended = 0,
    Timed = 1, // Departing vehicle is expected to wait
    MinimumTimeRequired = 2, // Time required is specified in min_transfer_time
    NotPossible = 3,
    InSeatTransfer = 4,
    MustReBoard = 5,
}

// Struct for transfers.txt
#[derive(Debug, Deserialize)]
pub struct Transfer {
    from_stop_id: u32,
    to_stop_id: u32,
    from_route_id: Option<u32>,
    to_route_id: Option<u32>,
    from_trip_id: Option<u32>,
    to_trip_id: Option<u32>,
    transfer_type: TransferType,
    min_transfer_time: Option<i32>,
}

// Struct for trips.txt
#[derive(Debug, Deserialize)]
pub struct Trip {
    route_id: u32,
    service_id: u32,
    trip_id: u32,
    realtime_trip_id: String,
    trip_headsign: String,
    trip_short_name: Option<String>,
    trip_long_name: Option<String>,
    direction_id: i32,
    block_id: Option<String>,
    shape_id: Option<u32>,
    wheelchair_accessible: u32,
    bikes_allowed: Option<u32>,
}