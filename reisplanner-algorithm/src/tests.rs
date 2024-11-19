use tracing_subscriber::EnvFilter;

use reisplanner_gtfs::utils::TimeTuple;
use crate::algorithms::csa;
use crate::algorithms::raptor;
use crate::database::new_db_connection;
use crate::getters::get_stop;

#[tokio::test]
async fn csa_algorithm() -> anyhow::Result<()>{
    let log_level = EnvFilter::try_from_default_env()
        // .unwrap_or(EnvFilter::new("error,reisplanner=debug,rbatis=debug"));
        .unwrap_or(EnvFilter::new("error,reisplanner=debug"));
    tracing_subscriber::fmt().with_env_filter(log_level).init();
    
    let db = &new_db_connection()?;
    let timetable = csa::get_timetable(db, true).await?;

    let cases = [
        (18124, 18305, TimeTuple(10, 00, 00)),
        (18124, 18004, TimeTuple(10, 00, 00)),
    ];
    
    for (departure, arrival, departure_time) in cases {
        let dep_name = get_stop(&departure, db).await?.stop_name;
        let arr_name = get_stop(&arrival, db).await?.stop_name;
        println!("Planning route between {dep_name} and {arr_name}");
        
        let result = csa::run_csa(
            departure, arrival, departure_time, &timetable
        ).await?;
        match result {
            None => {println!("No result found...")}
            Some(result) => {csa::print_result(&result, db).await?}
        }
        println!()
    }
    
    Ok(())
}

#[tokio::test]
async fn raptor_algorithm() -> anyhow::Result<()> {
    let log_level = EnvFilter::try_from_default_env()
        // .unwrap_or(EnvFilter::new("error,reisplanner=debug,rbatis=debug"));
        .unwrap_or(EnvFilter::new("error,reisplanner=debug"));
    tracing_subscriber::fmt().with_env_filter(log_level).init();

    let db = &new_db_connection()?;
    let timetable = raptor::get_timetable(db, true).await?;
    let transfers = raptor::generate_transfer_times(db).await?;

    let cases = [
        // (18124, 18305, TimeTuple(10, 00, 00)),
        (18124, 18004, TimeTuple(10, 00, 00)),
        (18124, 153150, TimeTuple(10, 00, 00)),
        // (18124, 18195, TimeTuple(10, 00, 00)),
        // (17843, 18029, TimeTuple(10, 00, 00)),
        // (17843, 449004, TimeTuple(10, 00, 00)),
    ];

    for (departure, arrival, departure_time) in cases {
        let dep_name = get_stop(&departure, db).await?.stop_name;
        let arr_name = get_stop(&arrival, db).await?.stop_name;
        println!("Planning route between {dep_name} and {arr_name}");

        let result = raptor::run_raptor(
            departure, arrival, departure_time, &timetable, &transfers, db
        ).await;
        match result {
            Err(e) => {eprintln!("Error: {e}")}
            Ok(None) => {println!("No result found...")}
            Ok(Some(result)) => {raptor::print_result(&result, db).await?}
        }
        println!()
    }
    
    Ok(())
}