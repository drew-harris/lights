use btleplug::{
    api::{
        bleuuid::BleUuid, Central, Characteristic, Manager as _, Peripheral as _, ScanFilter,
        WriteType::WithoutResponse,
    },
    platform::{Manager, Peripheral},
};
use colorsys::{Hsl, Rgb};
use image::ImageBuffer;
use nokhwa::*;
use std::{error::Error, time::Duration};
use tokio::time;

const UPDATE_LIGHTS: bool = true;
const SENSITIVITY: i32 = 3;
const SLOW_TRANSITION: bool = true;
const MULTI_COLOR: bool = false;

struct Light {
    device: Peripheral,
    charis: Characteristic,
    current_color: (u8, u8, u8),
}

impl Light {
    async fn set_color(&self, r: u8, g: u8, b: u8) -> Result<(), btleplug::Error> {
        let cmd: Vec<u8> = vec![0x33, 0x05, 0x02, r, g, b];
        self.send_raw_command(cmd).await.ok();
        Ok(())
    }

    async fn keep_alive(&self) -> Result<(), btleplug::Error> {
        let cmd: Vec<u8> = vec![0xAA, 0x01];
        self.send_raw_command(cmd).await.ok();
        Ok(())
    }

    async fn set_color_slowly(&mut self, r: u8, g: u8, b: u8) -> Result<(), btleplug::Error> {
        let target_color = (r, g, b);
        let current_color = self.current_color;

        if i32::abs(current_color.1 as i32 - target_color.1 as i32) < SENSITIVITY
            && i32::abs(current_color.2 as i32 - target_color.2 as i32) < SENSITIVITY
            && i32::abs(current_color.2 as i32 - target_color.2 as i32) < SENSITIVITY
        {
            return Ok(());
        }

        if SLOW_TRANSITION {
            let red: u8 = ((current_color.0 as i16 + target_color.0 as i16) / 2) as u8;
            let green: u8 = ((current_color.1 as i16 + target_color.1 as i16) / 2) as u8;
            let blue: u8 = ((current_color.2 as i16 + target_color.2 as i16) / 2) as u8;

            self.current_color = (red as u8, green as u8, blue as u8);
            let cmd: Vec<u8> = vec![0x33, 0x05, 0x02, red, green, blue];
            self.send_raw_command(cmd).await.ok();
            return Ok(());
        }
        self.current_color = (r as u8, g as u8, b as u8);
        let cmd: Vec<u8> = vec![0x33, 0x05, 0x02, r, g, b];
        self.send_raw_command(cmd).await.ok();
        Ok(())
    }

    async fn send_raw_command(&self, mut cmd: Vec<u8>) -> Result<(), btleplug::Error> {
        fill_and_sum(&mut cmd);
        self.device
            .write(&self.charis, &cmd, WithoutResponse)
            .await?;
        Ok(())
    }
}

async fn get_devices(match_names: Vec<String>) -> Result<Vec<Light>, Box<dyn Error>> {
    let mut lights = Vec::new();

    let manager = Manager::new().await.unwrap();

    // get the first bluetooth adapter
    let adapters = manager.adapters().await?;
    let central = adapters.into_iter().next().unwrap();
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

        println!("Found device: {}", name);

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
            p.discover_services().await?;

            let chars = p.characteristics();
            let char_cmd = match chars
                .iter()
                .find(|c| c.uuid.to_short_string() == *"00010203-0405-0607-0809-0a0b0c0d2b11")
            {
                Some(c) => c.clone(),
                None => {
                    continue;
                }
            };

            let light = Light {
                device: p,
                charis: char_cmd,
                current_color: (0, 0, 0),
            };

            lights.push(light);
        }
    }
    Ok(lights)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let matches = vec![
        "48EA".to_string(),
        "6072".to_string(),
        "6146".to_string(),
        // "6142".to_string(),
    ];
    let mut lights = match get_devices(matches).await {
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
        1,
        Some(CameraFormat::new_from(640, 480, FrameFormat::MJPEG, 30)),
    )
    .unwrap();

    // Loop unless a key is pressed
    let mut keep_alive_count = 0;

    loop {
        let image = camera.frame().unwrap();
        // Get average color
        let averages = get_average_color(image, lights.len() as u8);

        if keep_alive_count % 10 == 0 {
            for light in lights.iter_mut() {
                match light.keep_alive().await {
                    Ok(_) => {}
                    Err(_) => {
                        println!("Lost connection to light");
                    }
                }
            }
            keep_alive_count = 0;
        }
        keep_alive_count += 1;

        if UPDATE_LIGHTS {
            for (i, light) in lights.iter_mut().enumerate() {
                let index = match MULTI_COLOR {
                    true => i,
                    false => 0,
                };
                light
                    .set_color_slowly(averages[index].0, averages[index].1, averages[index].2)
                    .await?;
            }
        }
    }

    // for light in lights.iter() {
    //     light.device.disconnect().await?;
    // }

    // println!("Disconnected");

    // Ok(())
}

fn get_average_color(
    image: ImageBuffer<image::Rgb<u8>, std::vec::Vec<u8>>,
    mut num_colors: u8,
) -> Vec<(u8, u8, u8)> {
    // Use color-thief

    // Get sample of 100 pixels
    let mut colors = vec![];

    let sample = image.to_vec();

    if num_colors < 2 {
        num_colors = 2;
    }

    let pallette =
        color_thief::get_palette(&sample[..], color_thief::ColorFormat::Rgb, 1, num_colors)
            .unwrap();

    for color in pallette {
        let mut rgb: Rgb = (color.r, color.g, color.b).into();
        // Convert to HSV
        let mut hsl: Hsl = rgb.into();

        // Boost saturation
        hsl.set_saturation((hsl.saturation() * 1.5) + 32.0);

        // Convert back to RGB
        rgb = hsl.into();

        colors.push((rgb.red() as u8, rgb.green() as u8, rgb.blue() as u8));
    }
    colors
}

fn fill_and_sum(input_cmd: &mut Vec<u8>) {
    // Zero-pad and then add an XOR checksum
    while input_cmd.len() < 19 {
        input_cmd.push(0x00);
    }

    let mut sum = 0;
    for i in input_cmd.iter() {
        sum ^= i;
    }
    input_cmd.push(sum);
}
