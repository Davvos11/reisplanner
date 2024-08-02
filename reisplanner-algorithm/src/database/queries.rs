use std::collections::HashMap;
use std::str::FromStr;

use anyhow::Context;
use rbatis::RBatis;
use serde::Deserialize;

#[derive(Deserialize)]
struct StationParent {
    stop_id: String,
    parent_station: Option<String>
}

pub async fn get_parent_station_map(db: &RBatis) -> anyhow::Result<HashMap<u32, u32>> {
    let stops: Vec<StationParent> = db
        .query_decode("select stop_id, parent_station from stop", vec![])
        .await?;

    let mut map = HashMap::with_capacity(stops.len());

    for StationParent{ stop_id, parent_station } in stops {
        let stop_id = parse_stop_id(&stop_id)?;
        match parent_station {
            None => {
                map.insert(stop_id, stop_id);
            }
            Some(parent_id) => {
                let parent_id = parse_stop_id(&parent_id)?;
                map.insert(stop_id, parent_id);
            }
        }
    }

    Ok(map)
}

pub fn parse_stop_id(stop_id: &String) -> anyhow::Result<u32> {
    // stop_id is "stoparea:123456", so we parse to just 123456
    // we assume that no regular stop 123456 exists
    if stop_id.starts_with('s') {
        u32::from_str(&stop_id[9..])
            .context(format!("Tried to parse parent_station {stop_id}"))
    } else {
        u32::from_str(stop_id)
            .context(format!("Tried to parse parent_station {stop_id}"))
    }
}