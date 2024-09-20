use rbatis::executor::Executor;
use crate::algorithms::raptor::Connection;
use crate::getters::get_stop_readable;
use crate::utils::{seconds_to_hms, seconds_to_ms};

pub enum JourneyPart {
    Vehicle(Connection, Connection),
    Transfer(u32, u32, u32),
}

impl JourneyPart {
    pub async fn to_string(&self, db: &impl Executor) -> anyhow::Result<String> {
        match self {
            JourneyPart::Vehicle(connection_a, connection_b) => {
                Ok(
                    format!("{} @ {} - {} @ {} using {}",
                            connection_a.departure_name(db).await?,
                            seconds_to_hms(connection_a.departure),
                            connection_b.arrival_name(db).await?,
                            seconds_to_hms(connection_b.arrival),
                            connection_b.route_information(db).await?
                    )
                )
            }
            JourneyPart::Transfer(from, to, duration) => {
                Ok(
                    format!("Change from {} to {} ({} mins)",
                            get_stop_readable(from, db).await?,
                            get_stop_readable(to, db).await?,
                            seconds_to_ms(*duration),
                    )
                )
            }
        }
    }
}