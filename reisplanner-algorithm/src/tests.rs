use std::env;
use super::*;

#[tokio::test]
async fn it_works() -> anyhow::Result<()>{
     let path = env::current_dir()?;
    println!("The current directory is {}", path.display());
    generate_timetable().await?;
    
    Ok(())
}
