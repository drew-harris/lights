use btleplug::{platform::Manager, api::{Manager as _, Central, ScanFilter}};
use std::{error::Error, time};

async fn run() -> Result<(), Box<dyn Error>> {

    println!("AHHHH");
    let manager = Manager::new().await.unwrap();

    // get the first bluetooth adapter
    let adapters = manager.adapters().await?;

    let central = adapters.into_iter().nth(0).unwrap();

    central.start_scan(ScanFilter::default()).await.unwrap();


    Ok(())

}

fn main() {
    println!("Hello, world!");
    run();
    
}
