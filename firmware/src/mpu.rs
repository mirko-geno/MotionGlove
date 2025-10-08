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
};
use crate::{SensorReadings, CHANNEL_SIZE, READ_FREQ, MessageArr};


async fn calibrate_sensor(mpu: &mut Mpu6050<I2c<'static, I2C0, i2c::Async>>) {
    let calibration_params = CalibrationParameters::new(
        mpu6050_dmp::accel::AccelFullScale::G8,
        mpu6050_dmp::gyro::GyroFullScale::Deg500,
        mpu6050_dmp::calibration::ReferenceGravity::Zero,
    );

    log::info!("Calibrating Sensor");
    mpu.calibrate(&mut Delay, &calibration_params).await.unwrap();
    log::info!("Sensor Calibrated");
}


#[embassy_executor::task]
pub async fn read_mpu(mut mpu: Mpu6050<I2c<'static, I2C0, i2c::Async>>, tx_ch: Sender<'static, CriticalSectionRawMutex, MessageArr, CHANNEL_SIZE>) -> ! {
    /*
    let mut count = 0;
    loop {
        let mut message: String<MESSAGE_LENGTH> = String::new();
        write!(&mut message, "\nCount: {count}").unwrap();
        log::info!("\nCount: {count}");
        tx_ch.send(message).await;
        count += 1;
        Timer::after_millis(1).await;
    }
    */
    // Initialize DMP
    log::info!("Initializing DMP");
    mpu.initialize_dmp(&mut Delay).await.unwrap();

    // Calibrate sensor
    calibrate_sensor(&mut mpu).await;

    // Main loop reading quaternion data
    loop {
        let (accel, gyro) = (
            mpu.accel().await.unwrap(),
            mpu.gyro().await.unwrap()
        );
        log::info!("Sensor Readings:");
        log::info!(
            "  Accelerometer [mg]: x={}, y={}, z={}",
            accel.x(),
            accel.y(),
            accel.z()
        );
        log::info!(
            "  Gyroscope [deg/s]: x={}, y={}, z={}",
            gyro.x(),
            gyro.y(),
            gyro.z()
        );
        let readings = SensorReadings::from(accel, gyro);
        tx_ch.send(readings.as_bytes()).await;
        Timer::after(Duration::from_hz(READ_FREQ)).await;
    }
}

