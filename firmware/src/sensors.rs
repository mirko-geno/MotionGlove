use {defmt_rtt as _, panic_probe as _};
use embassy_time::{Delay, Duration, Timer};
use embassy_rp::{
    peripherals::I2C0, 
    i2c::{self, I2c},
};
use embassy_sync::{
    channel::Sender,
    blocking_mutex::raw::CriticalSectionRawMutex,
};
use mpu6050_dmp::{
    sensor_async::Mpu6050,
    calibration::CalibrationParameters,
    gyro::Gyro, accel::Accel
};
use usbd_hid::descriptor::{
    MouseReport,
    KeyboardReport, KeyboardUsage,
    MediaKeyboardReport, MediaKey,
};
use libm::{cos, sin, round};
use crate::{HidInstruction, CHANNEL_SIZE, READ_FREQ};

async fn calibrate_mpu(mpu: &mut Mpu6050<I2c<'static, I2C0, i2c::Async>>) {
    let calibration_params = CalibrationParameters::new(
        mpu6050_dmp::accel::AccelFullScale::G4,
        mpu6050_dmp::gyro::GyroFullScale::Deg2000,
        mpu6050_dmp::calibration::ReferenceGravity::Zero,
    );

    log::info!("Calibrating Sensor");
    mpu.calibrate(&mut Delay, &calibration_params).await.unwrap();
    log::info!("Sensor Calibrated");
}

pub async fn configure_mpu(mpu: &mut Mpu6050<I2c<'static, I2C0, i2c::Async>>) {
    // Initialize DMP
    log::info!("Initializing DMP");
    mpu.initialize_dmp(&mut Delay).await.unwrap();

    // Calibrate mpu
    calibrate_mpu(mpu).await;
}

async fn read_sensors(mpu: &mut Mpu6050<I2c<'static, I2C0, i2c::Async>>) -> (Accel, Gyro) {
    // Tries to get accel and gyro data from motion6, in case of error returns zeros
    let (accel, gyro) = match mpu.motion6().await {
        Err(e) => {
            log::warn!("Error {:?} while reading mpu", {e});
            (Accel::new(0,0,0), Gyro::new(0,0,0))
        }
        Ok(readings) => {
            readings
        }
    };
    log::info!("Sensor Readings:");
    log::info!("Accelerometer [mg]: x={}, y={}, z={}", accel.x(), accel.y(), accel.z());
    log::info!("Gyroscope [deg/s]: x={}, y={}, z={}", gyro.x(), gyro.y(), gyro.z());
    // tx_ch.send(readings.as_bytes()).await;
    (accel, gyro)
}

#[embassy_executor::task]
pub async fn sensor_processing(
    mut mpu: Mpu6050<I2c<'static, I2C0, i2c::Async>>,
    tx_ch: Sender<'static, CriticalSectionRawMutex, HidInstruction, CHANNEL_SIZE>
) -> ! {
    let mut pitch = 0.0;
    loop {
        // Read sensor data
        let (accel, gyro) = read_sensors(&mut mpu).await;

        // Process sensors
        let pitch_dot = (gyro.y() as f64) * cos(pitch) - (gyro.z() as f64) * sin(pitch);
        pitch = pitch + pitch_dot / READ_FREQ as f64;
        log::info!("pitch = {:?}", &pitch);

        // Make Hid reports from sensor processing
        let mouse_report = MouseReport {
            buttons: 0,
            x: 0,
            y: round(pitch) as i8,
            wheel: 0,
            pan: 0,
        };
        let keyboard_report = KeyboardReport {
            modifier: 0,
            reserved: 0,
            leds: 0,
            keycodes: [KeyboardUsage::KeyboardAa as u8, 0, 0, 0, 0, 0]
        };
        let media_report = MediaKeyboardReport {
            usage_id: MediaKey::PlayPause.into() // Pause / Play button
        };

        let hid_report = HidInstruction {
            mouse: mouse_report,
            keyboard: keyboard_report,
            media: media_report
        };

        tx_ch.send(hid_report).await;

        // Limit working frequency
        Timer::after(Duration::from_hz(READ_FREQ)).await;
    }

}