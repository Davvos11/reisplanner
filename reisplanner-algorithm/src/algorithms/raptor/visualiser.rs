use crate::algorithms::raptor::{Arrival, Connection};
use crate::getters::get_stop_readable;
use crate::utils::seconds_to_hms;
use dot_writer::{Attributes, Color, DotWriter, Style};
use rbatis::executor::Executor;
use std::collections::HashMap;

pub async fn visualise_earliest_arrivals(
    tau: &HashMap<u32, Arrival>,
    destination: u32,
    db: &impl Executor,
) -> anyhow::Result<()> {
    let mut output_bytes = Vec::new();
    {
        let mut writer = DotWriter::from(&mut output_bytes);

        let mut digraph = writer.digraph();

        for (id, arrival) in tau.iter() {
            let name = get_stop_readable(&arrival.stop.parent_id, db).await?;
            if arrival.departure_stop.is_some() {
                let from_station = get_stop_readable(&arrival.departure_stop.unwrap().parent_id, db).await?;
                // let route = c2.route_information(db).await?;
                let arrival = seconds_to_hms(arrival.time);

                digraph.edge(format!("\"{from_station}\""), format!("\"{name}\""))
                    .attributes().set_label(&format!("{arrival}"));
            } else {
                digraph.node_named(format!("\"{name}\""))
                    .set_color(Color::Red).set_style(Style::Filled);
            }
        }
        
        let destination_name = get_stop_readable(&destination, db).await?;
        digraph.node_named(format!("\"{destination_name}\""))
            .set_color(Color::PaleGreen).set_style(Style::Filled);
    }
    let result = String::from_utf8(output_bytes)?;
    println!("{}", result);

    Ok(())
}