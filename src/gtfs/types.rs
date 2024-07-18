use rbatis::executor::Executor;
use rbatis::{impl_select, impl_update};
use rbatis::rbdc::{Date, Error};
use rbatis::rbdc::db::ExecResult;
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

use crate::crud_trait;
use crate::rbatis_wrapper::DatabaseModel;
use crate::utils::{deserialize_date, deserialize_time_tuple, TimeTuple};

// Struct for agency.txt
#[derive(Debug, Deserialize, Serialize, Default)]
pub struct Agency {
    agency_id: String,
    agency_name: String,
    agency_url: String,
    agency_timezone: String,
    agency_phone: String,
}
crud_trait!(Agency {});

#[derive(Deserialize_repr, Serialize_repr, PartialEq, Debug, Default)]
#[repr(u8)]
enum ExceptionType {
    #[default]
    Added = 1,
    Removed = 2,
}

// Struct for calendar_dates.txt
#[derive(Debug, Deserialize, Serialize, Default)]
pub struct CalendarDate {
    service_id: u32,
    #[serde(deserialize_with = "deserialize_date")]
    date: Date,
    exception_type: ExceptionType,
}
crud_trait!(CalendarDate {});

// Struct for feed_info.txt
#[derive(Debug, Deserialize, Serialize, Default)]
pub struct FeedInfo {
    feed_publisher_name: String,
    feed_id: String,
    feed_publisher_url: String,
    feed_lang: String,
    #[serde(deserialize_with = "deserialize_date")]
    feed_start_date: Date,
    #[serde(deserialize_with = "deserialize_date")]
    feed_end_date: Date,
    feed_version: String,
}
crud_trait!(FeedInfo {});

#[derive(Deserialize_repr, Serialize_repr, PartialEq, Debug, Default)]
#[repr(u8)]
enum RouteType {
    #[default]
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
#[derive(Debug, Deserialize, Serialize)]
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

impl Default for Route {
    fn default() -> Self {
        Self {
            route_id: Default::default(),
            agency_id: Default::default(),
            route_short_name: Default::default(),
            route_long_name: Default::default(),
            route_desc: Some(Default::default()),
            route_type: Default::default(),
            route_color: Some(Default::default()),
            route_text_color: Some(Default::default()),
            route_url: Some(Default::default()),
        }
    }
}

crud_trait!(Route {});

// Struct for shapes.txt
#[derive(Debug, Deserialize, Serialize)]
pub struct Shape {
    shape_id: u32,
    shape_pt_sequence: u32,
    shape_pt_lat: f64,
    shape_pt_lon: f64,
    shape_dist_traveled: Option<f64>,
}

impl Default for Shape {
    fn default() -> Self {
        Self {
            shape_id: Default::default(),
            shape_pt_sequence: Default::default(),
            shape_pt_lat: Default::default(),
            shape_pt_lon: Default::default(),
            shape_dist_traveled: Some(Default::default()),
        }
    }
}

crud_trait!(Shape {});


#[derive(Deserialize_repr, Serialize_repr, Default, PartialEq, Debug)]
#[repr(u8)]
enum LocationType {
    #[default]
    Stop = 0, // (Platform when defined within a parent_station)
    Station = 1,
    Entrance = 2,
    GenericNode = 3,
    BoardingArea = 4,
}


#[derive(Deserialize_repr, Serialize_repr, Default, PartialEq, Debug)]
#[repr(u8)]
enum WheelchairBoarding {
    #[default]
    Empty = 0,
    SomePossible = 1,
    NotPossible = 2,
}

// Struct for stops.txt
#[derive(Debug, Deserialize, Serialize)]
pub struct Stop {
    stop_id: String,
    stop_code: Option<String>,
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

impl Default for Stop {
    fn default() -> Self {
        Self {
            stop_id: Default::default(),
            stop_code: Some(Default::default()),
            stop_name: Default::default(),
            stop_lat: Default::default(),
            stop_lon: Default::default(),
            location_type: Default::default(),
            parent_station: Some(Default::default()),
            stop_timezone: Some(Default::default()),
            wheelchair_boarding: Some(Default::default()),
            platform_code: Some(Default::default()),
            zone_id: Some(Default::default()),
        }
    }
}

crud_trait!(Stop {});


#[derive(Deserialize_repr, Serialize_repr, Default, PartialEq, Debug, Clone)]
#[repr(u8)]
pub enum PickupType {
    #[default]
    Regular = 0,
    NotAvailable = 1,
    MustPhone = 2,
    MustCoordinate = 3,
    // pickup_type=0 forbidden if start_pickup_drop_off_window or end_pickup_drop_off_window are defined.
    // pickup_type=3 forbidden if start_pickup_drop_off_window or end_pickup_drop_off_window are defined.
    // drop_off_type=0 forbidden if start_pickup_drop_off_window or end_pickup_drop_off_window are defined.
}

// Struct for stop_times.txt
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct StopTime {
    pub trip_id: u32,
    pub stop_sequence: u32,
    // TODO u32 instead of string?
    pub stop_id: String,
    pub stop_headsign: Option<String>,
    #[serde(deserialize_with = "deserialize_time_tuple")]
    pub arrival_time: TimeTuple,
    #[serde(deserialize_with = "deserialize_time_tuple")]
    pub departure_time: TimeTuple,
    pub pickup_type: PickupType,
    pub drop_off_type: PickupType,
    pub timepoint: i32,
    pub shape_dist_traveled: Option<f64>,
    pub fare_units_traveled: Option<i32>,
    // Fields added by realtime updates
    #[serde(default)]
    pub arrival_delay: Option<i32>,
    #[serde(default)]
    pub departure_delay: Option<i32>,
}

impl Default for StopTime {
    fn default() -> Self {
        Self {
            trip_id: Default::default(),
            stop_sequence: Default::default(),
            stop_id: Default::default(),
            stop_headsign: Some(Default::default()),
            arrival_time: Default::default(),
            departure_time: Default::default(),
            pickup_type: Default::default(),
            drop_off_type: Default::default(),
            timepoint: Default::default(),
            shape_dist_traveled: Some(Default::default()),
            fare_units_traveled: Some(Default::default()),
            arrival_delay: Some(Default::default()),
            departure_delay: Some(Default::default()),
        }
    }
}

crud_trait!(StopTime {});
impl_select!(StopTime {
    select_by_id_and_trip(stop_id:&u32,trip_id:&u32) => "`where stop_id = #{stop_id} and trip_id = #{trip_id}`"
});
impl_select!(StopTime {
    select_by_sequence_and_trip(stop_sequence:&u32,trip_id:&u32) => "`where stop_sequence = #{stop_sequence} and trip_id = #{trip_id}`"
});
impl_update!(StopTime {
    update_by_id_and_trip(stop_id:&u32,trip_id:&u32) => "`where stop_id = #{stop_id} and trip_id = #{trip_id}`"
});


#[derive(Deserialize_repr, Serialize_repr, Default, PartialEq, Debug)]
#[repr(u8)]
enum TransferType {
    #[default]
    Recommended = 0,
    Timed = 1, // Departing vehicle is expected to wait
    MinimumTimeRequired = 2, // Time required is specified in min_transfer_time
    NotPossible = 3,
    InSeatTransfer = 4,
    MustReBoard = 5,
}

// Struct for transfers.txt
#[derive(Debug, Deserialize, Serialize)]
pub struct Transfer {
    from_stop_id: String,
    to_stop_id: String,
    from_route_id: Option<u32>,
    to_route_id: Option<u32>,
    from_trip_id: Option<u32>,
    to_trip_id: Option<u32>,
    transfer_type: TransferType,
    min_transfer_time: Option<i32>,
}

impl Default for Transfer {
    fn default() -> Self {
        Self {
            from_stop_id: Default::default(),
            to_stop_id: Default::default(),
            from_route_id: Some(Default::default()),
            to_route_id: Some(Default::default()),
            from_trip_id: Some(Default::default()),
            to_trip_id: Some(Default::default()),
            transfer_type: Default::default(),
            min_transfer_time: Some(Default::default()),
        }
    }
}

crud_trait!(Transfer {});

#[derive(Deserialize_repr, Serialize_repr, Default, PartialEq, Debug)]
#[repr(u8)]
enum AllowedType {
    #[default]
    NoInformation = 0,
    Allowed = 1,
    NotAllowed = 2,
}


// Struct for trips.txt
#[derive(Debug, Deserialize, Serialize)]
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
    wheelchair_accessible: Option<AllowedType>,
    bikes_allowed: Option<AllowedType>,
    // Fields added by realtime updates:
    #[serde(default)]
    pub delay: Option<i32>,
}

impl Default for Trip {
    fn default() -> Self {
        Self {
            route_id: Default::default(),
            service_id: Default::default(),
            trip_id: Default::default(),
            realtime_trip_id: Default::default(),
            trip_headsign: Default::default(),
            trip_short_name: Some(Default::default()),
            trip_long_name: Some(Default::default()),
            direction_id: Default::default(),
            block_id: Some(Default::default()),
            shape_id: Some(Default::default()),
            wheelchair_accessible: Some(Default::default()),
            bikes_allowed: Some(Default::default()),
            delay: Some(Default::default()),
        }
    }
}

crud_trait!(Trip {});