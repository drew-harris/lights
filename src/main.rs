use btleplug::{
    api::{
        bleuuid::BleUuid, Central, Characteristic, Manager as _, Peripheral as _, ScanFilter,
        WriteType::WithoutResponse,
    },
    platform::{Adapter, Manager, Peripheral},
};
use image::ImageBuffer;
use nokhwa::*;
use std::{error::Error, time::Duration};
use tokio::time;

const UPDATE_LIGHTS: bool = true;

struct Light {
    device: Peripheral,
    charis: Characteristic,
}

impl Light {
    async fn set_color(&self, r: u8, g: u8, b: u8) -> Result<(), btleplug::Error> {
        let mut cmd: Vec<u8> = vec![0x33, 0x05, 0x02, r, g, b];
        self.send_raw_command(cmd).await?;
        Ok(())
    }

    async fn send_raw_command(&self, mut cmd: Vec<u8>) -> Result<(), btleplug::Error> {
        fill_and_sum(&mut cmd);
        self.device
            .write(&self.charis, &cmd, WithoutResponse)
            .await?;
        return Ok(());
    }
}

async fn get_devices(matchNames: Vec<String>) -> Result<Vec<Light>, Box<dyn Error>> {
    let mut lights = Vec::new();

    let manager = Manager::new().await.unwrap();

    // get the first bluetooth adapter
    let adapters = manager.adapters().await?;
    let central = adapters.into_iter().nth(0).unwrap();
    central.start_scan(ScanFilter::default()).await.unwrap();
    time::sleep(Duration::from_secs(1)).await;

    for p in central.peripherals().await.unwrap() {
        let name = p.properties().await.unwrap().unwrap().local_name;
        let name = match name {
            Some(got_name) => got_name,
            None => {
                continue;
            }
        };

        let mut matched = false;

        for matchCode in matchNames.iter() {
            if name.contains(matchCode) {
                matched = true;
            }
        }

        if matched {
            p.connect().await.unwrap();
            println!("Connected");
            p.discover_services().await?;

            let chars = p.characteristics();
            let char_cmd = chars
                .iter()
                .find(|c| {
                    c.uuid.to_short_string() == "00010203-0405-0607-0809-0a0b0c0d2b11".to_string()
                })
                .unwrap()
                .clone();

            let light = Light {
                device: p,
                charis: char_cmd,
            };

            lights.push(light);
        }
    }
    return Ok(lights);
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let matches = vec!["48EA".to_string(), "6072".to_string(), "6146".to_string()];
    let lights = match get_devices(matches).await {
        Ok(lights) => lights,
        Err(_err) => panic!("Could not get devices"),
    };

    println!("Found {} Lights", lights.len());

    let mut camera = Camera::new(
        0,
        Some(CameraFormat::new_from(640, 480, FrameFormat::MJPEG, 30)),
    )
    .unwrap();

    // Loop unless a key is pressed

    loop {
        let image = camera.frame().unwrap();
        // Get average color
        let average = get_average_color(image);
        if average.1 < 4 {
            break;
        }

        if UPDATE_LIGHTS {
            for light in lights.iter() {
                light.set_color(average.0, average.1, average.2).await?;
            }
        }
        std::thread::sleep(Duration::from_millis(20));
    }

    for light in lights.iter() {
        light.device.disconnect().await?;
    }

    println!("Disconnected");

    Ok(())
}

fn get_average_color(image: ImageBuffer<image::Rgb<u8>, std::vec::Vec<u8>>) -> (u8, u8, u8) {
    // Use color-thief
    let pallette =
        color_thief::get_palette(&image.into_vec(), color_thief::ColorFormat::Rgb, 3, 2).unwrap();

    println!(
        "R: {}, G: {}, B: {}",
        pallette[0].r, pallette[0].g, pallette[0].b
    );
    return (pallette[0].r, pallette[0].g, pallette[0].b);
}

fn fill_and_sum(input_cmd: &mut Vec<u8>) {
    // Zero-pad and then add an XOR checksum
    while input_cmd.len() < 19 {
        input_cmd.push(0x00);
    }

    let mut sum = 0;
    for i in input_cmd.iter() {
        sum = sum ^ i;
    }
    input_cmd.push(sum);
}
