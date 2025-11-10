use core::f32::consts::PI;

use {defmt_rtt as _, panic_probe as _};
use embassy_time::{
    Duration, Timer, Instant,
    //Delay,
};
use embassy_rp::{
    peripherals::I2C0,
    gpio::Input,
    i2c::{self, I2c},
};
use embassy_sync::{
    channel::Sender,
    blocking_mutex::raw::CriticalSectionRawMutex,
};
use mpu9250_async::{
    sensor_async::Mpu9250,
    // calibration::CalibrationParameters,
    gyro::Gyro, accel::Accel, magnetometer::Mag
};
use usbd_hid::descriptor::{
    MouseReport,
    KeyboardReport, // KeyboardUsage,
    MediaKeyboardReport, MediaKey,
};
use libm::{atan2f, powf, sqrtf, roundf};

use shared::{
    definitions::{
        READ_FREQ, DELTA_TIME, PADDING_FREQ,
        CHANNEL_SIZE,
        DEAD_ZONE,
        ROLL_SENS, PITCH_SENS, WHEEL_SENS, PAN_SENS,
    },
    custom_hid::HidInstruction,
};

use crate::flexes::{
    FingerFlexes,FingerReadings,
    THUMB, INDEX, MIDDLE,
};

const OPENED: bool = false;
const CLOSED: bool = true;
const LEFT_CLICK: u8 = 1;
const RIGHT_CLICK: u8 = 2;


/*
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
*/

/*
pub async fn configure_mpu(mpu: &mut Mpu9250<I2c<'static, I2C0, i2c::Async>>) {
    // Initialize DMP
    log::info!("Initializing DMP");
    // mpu.initialize_dmp(&mut Delay).await.unwrap();

    // Calibrate mpu
    // calibrate_mpu(mpu).await;
}
*/

fn get_hid_report(
    vel_x: f32, vel_y: f32,
    finger_states: &[bool; 3],
    tap: bool,
    last_padding: &mut Instant
) -> HidInstruction {
    // Make Hid reports from sensor processing
    let mut mouse_report = MouseReport { buttons: 0, x:0, y:0, wheel: 0, pan: 0 };
    match tap {
        false => {
            mouse_report.x = roundf(vel_x * ROLL_SENS) as i8;
            mouse_report.y = roundf(vel_y * PITCH_SENS) as i8;
        },
        true => {
            let refresh = Duration::from_hz(PADDING_FREQ);
            if last_padding.elapsed() >= refresh {
                mouse_report.wheel = roundf(-vel_y * WHEEL_SENS) as i8;
                mouse_report.pan = roundf(vel_x * PAN_SENS) as i8;

                *last_padding = last_padding.saturating_add(refresh);
            }
            // Continue if not enough time elapsed
        }
    };
    if finger_states[INDEX] {mouse_report.buttons = LEFT_CLICK};
    if finger_states[MIDDLE] {mouse_report.buttons = RIGHT_CLICK};
    
    let keyboard_report = KeyboardReport {
        modifier: 0,
        reserved: 0,
        leds: 0,
        keycodes: [0, 0, 0, 0, 0, 0]
    };
    let media_report = MediaKeyboardReport {
        usage_id: MediaKey::Zero.into() // Pause / Play button
    };

    HidInstruction {
        mouse: mouse_report,
        keyboard: keyboard_report,
        media: media_report
    }
}


async fn read_sensors(
    mpu: &mut Mpu9250<I2c<'static, I2C0, i2c::Async>>,
    finger_flexes: &mut FingerFlexes<'static>,
    finger_tap: &mut Input<'static>
) -> (Accel, Gyro, FingerReadings, bool) {
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
    let tap = finger_tap.is_high();
    log::info!("Sensor Readings:");
    log::info!("Thumb: {}\nIndex: {}\nMiddle: {}\n", flexes[THUMB], flexes[INDEX], flexes[MIDDLE]);
    log::info!("Thumb and Index are touching: {}", tap);
    log::info!("Accelerometer [mg]: x={}, y={}, z={}", accel.x(), accel.y(), accel.z());
    log::info!("Gyroscope [deg/s]: x={}, y={}, z={}", gyro.x(), gyro.y(), gyro.z());
    log::info!("Magnetometer [algo]: x={}, y={}, z={}", mag.x(), mag.y(), mag.z());

    (accel, gyro, flexes, tap)
}


#[embassy_executor::task]
pub async fn sensor_processing(
    mut mpu: Mpu9250<I2c<'static, I2C0, i2c::Async>>,
    mut finger_flexes: FingerFlexes<'static>,
    mut finger_tap: Input<'static>,
    tx_ch: Sender<'static, CriticalSectionRawMutex, HidInstruction, CHANNEL_SIZE>
) -> ! {
    // Schmitt Trigger bands
    const SUP_BAND: u16 = 900;
    const LOW_BAND: u16 = 500;
    // Current flexes states
    let mut finger_states: [bool; 3] = [OPENED; 3];

    // MPU calculation constants:
    const ALPHA_ACC: f32 = 0.05;    // Relative weight of the accelerometer compared to the gyroscope
    // MPU variants:
    let mut pitch: f32;
    let mut roll: f32;
    let mut angle_x: f32 = 0.0;
    let mut angle_y: f32 = 0.0;

    // Mouse padding delay:
    let mut last_padding = Instant::now();
    loop {
        // Read sensor data
        let (accel, gyro, flexes, tap) = read_sensors(&mut mpu, &mut finger_flexes, &mut finger_tap).await;

        // Schmitt Trigger implemented for fingers
        for (idx, flex) in flexes.iter().enumerate() {
            finger_states[idx] = 
                if *flex >= SUP_BAND { OPENED }
                else if *flex <= LOW_BAND { CLOSED }
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

        angle_x = (1.0 - ALPHA_ACC) * (angle_x + gyro.x() as f32 * DELTA_TIME) + ALPHA_ACC * roll;
        angle_y = (1.0 - ALPHA_ACC) * (angle_y + gyro.y() as f32 * DELTA_TIME) + ALPHA_ACC * pitch;

        let vel_x = match angle_x.abs() > DEAD_ZONE {
            false => 0.0,
            true => angle_x.signum() * DELTA_TIME * powf(angle_x.abs() - DEAD_ZONE, 1.2)
        };
        let vel_y = match angle_y.abs() > DEAD_ZONE {
            false => 0.0,
            true => angle_y.signum() * DELTA_TIME * powf(angle_y.abs() - DEAD_ZONE, 1.2)
        };
        log::info!("vel_x: {}, vel_y: {}", vel_x, vel_y);

        // Get hid combination from sensors and send it to tcp client
        let hid_report = get_hid_report(vel_x, vel_y, &finger_states, tap, &mut last_padding);
        tx_ch.send(hid_report).await;

        // Limit working frequency
        Timer::after(Duration::from_hz(READ_FREQ)).await;
    }
}