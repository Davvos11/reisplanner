use tracing_subscriber::EnvFilter;
use reisplanner_gtfs::utils::TimeTuple;

use crate::algorithms::csa::{get_timetable, print_result, run_csa};
use crate::database::new_db_connection;
use crate::getters::get_stop_str;

#[tokio::test]
async fn csa_algorithm() -> anyhow::Result<()>{
    let log_level = EnvFilter::try_from_default_env()
        .unwrap_or(EnvFilter::new("error,reisplanner=debug"));
    tracing_subscriber::fmt().with_env_filter(log_level).init();
    
    let db = &new_db_connection()?;
    let timetable = get_timetable(db, false).await?;

    let cases = [
        ("stoparea:18124".to_string(), "stoparea:18305".to_string(), TimeTuple(10, 00, 00)),
        ("stoparea:18124".to_string(), "stoparea:18004".to_string(), TimeTuple(10, 00, 00)),
    ];
    
    for (departure, arrival, departure_time) in cases {
        let dep_name = get_stop_str(&departure, db).await?.stop_name;
        let arr_name = get_stop_str(&arrival, db).await?.stop_name;
        println!("Planning route between {dep_name} and {arr_name}");
        
        let result = run_csa(
            &departure, &arrival, departure_time, &timetable
        ).await?;
        match result {
            None => {println!("No result found...")}
            Some(result) => {print_result(&result, db).await?}
        }
        println!()
    }
    
    Ok(())
}
