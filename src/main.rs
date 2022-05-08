use btleplug::{
    api::{bleuuid::BleUuid, Central, Manager as _, Peripheral as _, ScanFilter},
    platform::{Adapter, Manager, Peripheral},
};
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
        let name = p.properties().await.unwrap().unwrap().local_name;
        match name {
            Some(name) => println!("{}", name),
            None => (),
        }
    }

    let light = find_light(&central).await.unwrap();

    println!(
        "Connecting to {}",
        light
            .properties()
            .await
            .unwrap()
            .unwrap()
            .local_name
            .unwrap()
    );
    light.connect().await.unwrap();
    println!("Connected");
    light.discover_services().await?;
    println!("Discovering services...");
    let chars = light.characteristics();
    for charis in chars.iter() {
        println!("{}", &charis.uuid.to_short_string());
    }

    let char_cmd = chars
        .iter()
        .find(|c| c.uuid.to_short_string() == "00010203-0405-0607-0809-0a0b0c0d2b11".to_string())
        .unwrap();

    let mut off_cmd: Vec<u8> = vec![0x33, 0x01, 0x00];
    let mut red: Vec<u8> = vec![0x33, 0x05, 0x02, 244, 0, 0];
    fill_and_sum(&mut off_cmd);
    fill_and_sum(&mut red);

    let result = light
        .write(
            &char_cmd,
            &red,
            btleplug::api::WriteType::WithoutResponse,
        )
        .await;
    match result {
        Ok(()) => println!("Command sent"),
        Err(err) => (),
    }

    time::sleep(Duration::from_secs(3)).await;

    light.disconnect().await?;
    println!("Disconnected");

    Ok(())
}

fn fill_and_sum(inputCmd: &mut Vec<u8>) {
    // Zeropad and then add an XOR checksum
    while inputCmd.len() < 19 {
        inputCmd.push(0x00);
    }

    let mut sum = 0;
    for i in inputCmd.iter() {
        sum = sum ^ i;
    }
    inputCmd.push(sum);
    println!("Command IS: {}", hex::encode(inputCmd));
}

async fn find_light(central: &Adapter) -> Option<Peripheral> {
    for p in central.peripherals().await.unwrap() {
        if p.properties()
            .await
            .unwrap()
            .unwrap()
            .local_name
            .iter()
            .any(|name| name.contains("6146"))
        {
            return Some(p);
        }
    }
    None
}
