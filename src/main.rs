use btleplug::{platform::{Manager, Adapter, Peripheral}, api::{Manager as _, Central, ScanFilter, Peripheral as _}};
use std::{error::Error, time::Duration};
use tokio::time;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {

    println!("Scanning devices");
    let manager = Manager::new().await.unwrap();

    // get the first bluetooth adapter
    let adapters = manager.adapters().await?;

    let central = adapters.into_iter().nth(0).unwrap();

    central.start_scan(ScanFilter::default()).await.unwrap();

    time::sleep(Duration::from_secs(1)).await;

    for p in central.peripherals().await.unwrap() {
        let name  = p.properties().await.unwrap().unwrap().local_name;
        match name {
            Some(name) => println!("{}", name),
            None => (),
        }
    }

    let light = find_light(&central).await.unwrap();

    Ok(())

}

async fn find_light(central: &Adapter) -> Option<Peripheral> {
    for p in central.peripherals().await.unwrap() {
        if p.properties()
            .await
            .unwrap()
            .unwrap()
            .local_name
            .iter()
            .any(|name| name.contains("48EA"))
        {
            return Some(p);
        }
    }
    None
}
