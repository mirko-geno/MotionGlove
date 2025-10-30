use core::f32::consts::PI;

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
use mpu9250_async::{
    sensor_async::Mpu9250,
    calibration::CalibrationParameters,
    gyro::Gyro, accel::Accel, magnetometer::Mag
};
use usbd_hid::descriptor::{
    MouseReport,
    KeyboardReport, KeyboardUsage,
    MediaKeyboardReport, MediaKey,
};
use libm::{atan2f, powf, sqrtf, roundf};
use crate::{FingerFlexes, FingerReadings, HidInstruction, THUMB, INDEX, MIDDLE, CHANNEL_SIZE, READ_FREQ, DELTA_TIME, DEAD_ZONE, PX_SENS};

async fn calibrate_mpu(mpu: &mut Mpu9250<I2c<'static, I2C0, i2c::Async>>) {
    let calibration_params = CalibrationParameters::new(
        mpu9250_async::accel::AccelFullScale::G4,
        mpu9250_async::gyro::GyroFullScale::Deg2000,
        mpu9250_async::calibration::ReferenceGravity::Zero,
    );

    log::info!("Calibrating Sensor");
    mpu.calibrate(&mut Delay, &calibration_params).await.unwrap();
    log::info!("Sensor Calibrated");
}

pub async fn configure_mpu(mpu: &mut Mpu9250<I2c<'static, I2C0, i2c::Async>>) {
    // Initialize DMP
    log::info!("Initializing DMP");
    // mpu.initialize_dmp(&mut Delay).await.unwrap();

    // Calibrate mpu
    // calibrate_mpu(mpu).await;
}

async fn read_sensors(
    mpu: &mut Mpu9250<I2c<'static, I2C0, i2c::Async>>,
    finger_flexes: &mut FingerFlexes<'static>,
) -> (Accel, Gyro, FingerReadings) {
    // Tries to get accel and gyro data from motion6, in case of error returns zeros
    let (accel, gyro, mag) = match mpu.motion9().await {
        Err(e) => {
            log::warn!("Error {:?} while reading mpu", {e});
            (Accel::new(0,0,0), Gyro::new(0,0,0), Mag::new(0, 0, 0))
        }
        Ok(readings) => {
            readings
        }
    };
    let flexes = match finger_flexes.read().await {
        Err(e) => {
            log::warn!("Error {:?} while reading ADC", {e});
            [0,0,0] // Return empty readings
        }
        Ok(readings) => readings
    };
    log::info!("Sensor Readings:");
    log::info!("Thumb: {}\nIndex: {}\nMiddle: {}\n", flexes[THUMB], flexes[INDEX], flexes[MIDDLE]);
    log::info!("Accelerometer [mg]: x={}, y={}, z={}", accel.x(), accel.y(), accel.z());
    log::info!("Gyroscope [deg/s]: x={}, y={}, z={}", gyro.x(), gyro.y(), gyro.z());
    log::info!("Magnetometer [algo]: x={}, y={}, z={}", mag.x(), mag.y(), mag.z());
    // tx_ch.send(readings.as_bytes()).await;
    (accel, gyro, flexes)
}

#[embassy_executor::task]
pub async fn sensor_processing(
    mut mpu: Mpu9250<I2c<'static, I2C0, i2c::Async>>,
    mut finger_flexes: FingerFlexes<'static>,
    tx_ch: Sender<'static, CriticalSectionRawMutex, HidInstruction, CHANNEL_SIZE>
) -> ! {
    // Schmitt Trigger bands
    const SUP_BAND: u16 = 900;
    const LOW_BAND: u16 = 500;
    const OPENED: bool = false;
    const CLOSED: bool = true;
    // Current flexes states
    let mut finger_states: [bool; 3] = [OPENED; 3];

    // MPU calculation constants:
    const ALPHA_ACC: f32 = 0.05;    // Relative weight of the accelerometer compared to the gyroscope
    // MPU variants:
    let mut pitch: f32;
    let mut roll: f32;
    let mut angle_x: f32 = 0.0;
    let mut angle_y: f32 = 0.0;
    loop {
        // Read sensor data
        let (accel, gyro, flexes) = read_sensors(&mut mpu, &mut finger_flexes).await;

        // Schmitt Trigger implemented for fingers
        for (idx, flex) in flexes.iter().enumerate() {
            finger_states[idx] = 
                if flex >= &SUP_BAND { OPENED }
                else if flex <= &LOW_BAND { CLOSED }
                else { finger_states[idx] };
        }

        log::info!("Thumb [bool]: {}\nIndex [bool]: {}\nMiddle [bool]: {}\n",
        finger_states[THUMB], finger_states[INDEX], finger_states[MIDDLE]);

        // Process mpu
        roll = atan2f(
            accel.y().into(),
            accel.z().into()) * 180.0 / PI;

        pitch = atan2f(
            (-1*accel.x()).into(),
            sqrtf(( accel.y() as i32 * accel.y() as i32 + accel.z() as i32 * accel.z() as i32 ) as f32 )) * 180.0 / PI;

        angle_x = (1.0 - ALPHA_ACC) * (angle_x + gyro.x() as f32 *DELTA_TIME) + ALPHA_ACC * roll;
        angle_y = (1.0 - ALPHA_ACC) * (angle_y + gyro.y() as f32 *DELTA_TIME) + ALPHA_ACC * pitch;

        let mut vel_x: f32 = 0.0;
        let mut vel_y: f32 = 0.0;
        if angle_x.abs() > DEAD_ZONE {
            vel_x = angle_x.signum() * PX_SENS * powf(angle_x.abs() - DEAD_ZONE, 1.2);            
        }
        if angle_y.abs() > DEAD_ZONE {
            vel_y = angle_y.signum() * PX_SENS * powf(angle_y.abs() - DEAD_ZONE, 1.2);            
        }

        log::info!("vel_x: {vel_x}, pitch: {pitch}");

        // Make Hid reports from sensor processing
        let mouse_report = MouseReport {
            buttons: 0,
            x: roundf(vel_x * DELTA_TIME) as i8,
            y: roundf(vel_y * DELTA_TIME) as i8,
            wheel: 0,
            pan: 0,
        };
        let keyboard_report = KeyboardReport {
            modifier: 0,
            reserved: 0,
            leds: 0,
            keycodes: [0, 0, 0, 0, 0, 0]
        };
        let media_report = MediaKeyboardReport {
            usage_id: MediaKey::Zero.into() // Pause / Play button
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