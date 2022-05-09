use btleplug::{
    api::{
        bleuuid::BleUuid, Central, Characteristic, Manager as _, Peripheral as _, ScanFilter,
        WriteType::WithoutResponse,
    },
    platform::{Manager, Peripheral},
};
use colorsys::{ColorTransform, ColorTuple, Hsl, Rgb};
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
        let cmd: Vec<u8> = vec![0x33, 0x05, 0x02, r, g, b];
        self.send_raw_command(cmd).await.ok();
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

async fn get_devices(match_names: Vec<String>) -> Result<Vec<Light>, Box<dyn Error>> {
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

        for match_code in match_names.iter() {
            if name.contains(match_code) {
                matched = true;
            }
        }

        if matched {
            // Connect or continue if failed
            let result = p.connect().await;
            match result {
                Ok(_device) => {
                    print!("Connected");
                }
                Err(_e) => {
                    continue;
                }
            };
            println!("Connected");
            p.discover_services().await?;

            let chars = p.characteristics();
            let char_cmd = match chars.iter().find(|c| {
                c.uuid.to_short_string() == "00010203-0405-0607-0809-0a0b0c0d2b11".to_string()
            }) {
                Some(c) => c.clone(),
                None => {
                    continue;
                }
            };

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
    let matches = vec![
        "48EA".to_string(),
        "6072".to_string(),
        "6146".to_string(),
        // "6142".to_string(),
    ];
    let lights = match get_devices(matches).await {
        Ok(lights) => lights,
        Err(_err) => panic!("Could not get devices"),
    };

    println!("Found {} Lights", lights.len());

    for light in lights.iter() {
        light.set_color(255, 0, 0).await.unwrap();
    }
    std::thread::sleep(std::time::Duration::from_millis(400));
    for light in lights.iter() {
        light.set_color(0, 255, 0).await.unwrap();
    }
    std::thread::sleep(std::time::Duration::from_millis(400));
    for light in lights.iter() {
        light.set_color(0, 0, 255).await.unwrap();
    }
    std::thread::sleep(std::time::Duration::from_millis(400));

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

    // Get sample of 100 pixels
    let mut sample = Vec::new();

    for x in (0..image.width()).step_by(10) {
        for y in (0..image.height()).step_by(10) {
            let pixel = image.get_pixel(x, y);
            sample.push(pixel[0]);
            sample.push(pixel[1]);
            sample.push(pixel[2]);
        }
    }

    let pallette =
        color_thief::get_palette(&sample[..], color_thief::ColorFormat::Rgb, 3, 2).unwrap();
    // color_thief::get_palette(&image.into_vec(), color_thief::ColorFormat::Rgb, 3, 2).unwrap();

    println!(
        "R: {}, G: {}, B: {}",
        pallette[0].r, pallette[0].g, pallette[0].b
    );

    let mut rgb: Rgb = (pallette[0].r, pallette[0].g, pallette[0].b).into();
    // Convert to HSV
    let mut hsl: Hsl = rgb.into();

    // Boost saturation
    hsl.set_saturation(hsl.saturation() * 1.5);
    if (hsl.saturation() > 210.0) {
        hsl.set_saturation(210.0);
    }

    // Convert back to RGB
    rgb = hsl.into();

    return (rgb.red() as u8, rgb.green() as u8, rgb.blue() as u8);
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
