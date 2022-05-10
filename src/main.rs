use btleplug::{
    api::{
        bleuuid::BleUuid, Central, Characteristic, Manager as _, Peripheral as _, ScanFilter,
        WriteType::WithoutResponse,
    },
    platform::{Manager, Peripheral},
};
use colorsys::{Hsl, Rgb};
use opencv::{highgui, prelude::*, videoio, Result};
use std::{error::Error, time::Duration};
use tokio::time;

const UPDATE_LIGHTS: bool = true;

struct Light {
    device: Peripheral,
    charis: Characteristic,
    current_color: (u8, u8, u8),
}

impl Light {
    async fn set_color(&mut self, r: u8, g: u8, b: u8) -> Result<(), btleplug::Error> {
        let cmd: Vec<u8> = vec![0x33, 0x05, 0x02, r, g, b];

        self.current_color = (r, g, b);
        self.send_raw_command(cmd).await.ok();
        Ok(())
    }

    async fn set_color_slowly(&mut self, r: u8, g: u8, b: u8) -> Result<(), btleplug::Error> {
        let target_color = (r, g, b);
        let current_color = self.current_color;

        let red: u8 = ((current_color.0 as i16 + target_color.0 as i16) / 2) as u8;
        let green: u8 = ((current_color.1 as i16 + target_color.1 as i16) / 2) as u8;
        let blue: u8 = ((current_color.2 as i16 + target_color.2 as i16) / 2) as u8;

        self.current_color = (red as u8, green as u8, blue as u8);
        let cmd: Vec<u8> = vec![0x33, 0x05, 0x02, red, green, blue];
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

            let mut light = Light {
                device: p,
                charis: char_cmd,
                current_color: (0, 0, 0),
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
    let mut lights = match get_devices(matches).await {
        Ok(lights) => lights,
        Err(_err) => panic!("Could not get devices"),
    };

    println!("Found {} Lights", lights.len());

    for light in lights.iter_mut() {
        light.set_color(255, 0, 0).await.unwrap();
    }
    std::thread::sleep(std::time::Duration::from_millis(100));
    for light in lights.iter_mut() {
        light.set_color(0, 255, 0).await.unwrap();
    }
    std::thread::sleep(std::time::Duration::from_millis(200));
    for light in lights.iter_mut() {
        light.set_color(0, 0, 255).await.unwrap();
    }

    // List video devices with nokhwa

    let mut cam = videoio::VideoCapture::new(1, videoio::CAP_ANY).unwrap();

    // Loop unless a key is pressed

    loop {
        // Grab a frame and display it
        let mut frame = Mat::default();
        cam.read(&mut frame).unwrap();
        highgui::imshow("test", &frame).unwrap();

        let before = time::Instant::now();
        let average = get_average_color(frame, lights.len() as u8);

        println!("{:?} GET COLOR", before.elapsed());

        let before = std::time::Instant::now();
        if UPDATE_LIGHTS {
            for light in lights.iter_mut() {
                light
                    .set_color_slowly(average.0, average.1, average.2)
                    .await?;
            }
        }
        println!("{:?} UPDATE LIGHTS", before.elapsed());

        // Break on ESC key
        let key = highgui::wait_key(100)?;
        if key == 27 {
            break;
        }
    }

    for light in lights.iter() {
        light.device.disconnect().await?;
    }

    println!("Disconnected");

    Ok(())
}

fn get_average_color(image: Mat, num_colors: u8) -> (u8, u8, u8) {
    let frame2 = image.data_bytes().unwrap();

    let pallette =
        color_thief::get_palette(&frame2, color_thief::ColorFormat::Rgb, 10, num_colors).unwrap();
    let color: (u8, u8, u8) = (pallette[0].b, pallette[0].g, pallette[0].r);
    // Boost saturation
    let rgb = colorsys::Rgb::from(color);
    let mut hsl = Hsl::from(rgb);
    hsl.set_saturation(hsl.saturation() * 1.5);
    if hsl.saturation() > 99.0 {
        hsl.set_saturation(99.0);
    }
    let rgb = Rgb::from(hsl);

    (rgb.red() as u8, rgb.green() as u8, rgb.blue() as u8)
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
