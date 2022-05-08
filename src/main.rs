use btleplug::{platform::{Manager, Adapter, Peripheral}, api::{Manager as _, Central, ScanFilter, Peripheral as _, bleuuid::BleUuid}};
use std::{error::Error, time::Duration};
use tokio::time;


enum LightCommand {
    TurnOn()
}


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


    println!("Connecting to {}", light.properties().await.unwrap().unwrap().local_name.unwrap());
    light.connect().await.unwrap();
    println!("Connected");
    light.discover_services().await?;
    println!("Discovering services...");
    let chars = light.characteristics();
    for charis in chars.iter() {
        println!("{}", &charis.uuid.to_short_string());
        let off_cmd = vec![0x33, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x32];

        let keep_alive = vec![0xAA, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xAB];

        light.write(&charis, &keep_alive, btleplug::api::WriteType::WithoutResponse).await;
        light.write(&charis, &off_cmd, btleplug::api::WriteType::WithoutResponse).await;
    }

    //let char_cmd = chars.iter().find(|c| c.uuid.to_short_string() == "00010203-0405-0607-0809-0a0b0c0d2b11".to_string()).unwrap();

    //let off_cmd = vec![0x33, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x32];

    //let keep_alive = vec![0xAA, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xAB];

    //let result = light.write(&char_cmd, &keep_alive, btleplug::api::WriteType::WithoutResponse).await;
    //let result = light.write(&char_cmd, &off_cmd, btleplug::api::WriteType::WithoutResponse).await;
    //match result {
        //Ok(()) => println!("Command sent"),
        //Err(err) => (),
    //}



    time::sleep(Duration::from_secs(3)).await;


    light.disconnect().await?;
    println!("Disconnected");

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
