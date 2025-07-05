use {defmt_rtt as _, panic_probe as _};
use embassy_time::{Timer, Delay};
use embassy_rp::{
    peripherals::I2C0, 
    i2c::{self, I2c},
};
use mpu6050_dmp::{
    sensor_async::Mpu6050,
    calibration::CalibrationParameters,
    quaternion::Quaternion,
};


async fn calibrate_sensor(mpu: &mut Mpu6050<I2c<'static, I2C0, i2c::Async>>) {
    let calibration_params = CalibrationParameters::new(
        mpu6050_dmp::accel::AccelFullScale::G4,
        mpu6050_dmp::gyro::GyroFullScale::Deg2000,
        mpu6050_dmp::calibration::ReferenceGravity::XP,
    );

    log::info!("Calibrating Sensor");
    mpu.calibrate(&mut Delay, &calibration_params).await.unwrap();
    log::info!("Sensor Calibrated");
}


#[embassy_executor::task]
pub async fn read_mpu(mut mpu: Mpu6050<I2c<'static, I2C0, i2c::Async>>) -> ! {
    // Initialize DMP
    log::info!("Initializing DMP");
    mpu.initialize_dmp(&mut Delay).await.unwrap();

    // Calibrate sensor
    calibrate_sensor(&mut mpu).await;

    // Configure DMP update rate
    mpu.set_sample_rate_divider(2).await.unwrap(); // 2 for good motion tracking
    log::info!("Sample rate configured");


    // Enable FIFO for quaternion data
    mpu.enable_fifo().await.unwrap();
    // Buffer for FIFO data (DMP packets are 28 bytes)
    let mut buffer = [0u8; 28];

    // Main loop reading quaternion data
    loop {
        let fifo_count = mpu.get_fifo_count().await.unwrap();

        if fifo_count >= 28 {
            // Read a complete DMP packet
            let data = mpu.read_fifo(&mut buffer).await.unwrap();

            // First 16 bytes contain quaternion data
            // The quaternion represents the sensor's orientation in 3D space:
            // - w: cos(angle/2) - indicates amount of rotation
            // - i,j,k: axis * sin(angle/2) - indicates rotation axis
            let mut quat = Quaternion::from_bytes(&data[..16]).unwrap().normalize();

            /*
            In this part of the code the magnetometer data
            should be added to z component (k) of the quaternion
            */           

            // Display quaternion components
            // Values are normalized (sum of squares = 1)
            log::info!("\nQuaternion: w={:.3}, i={:.3}, j={:.3}, k={:.3}",quat.w, quat.x, quat.y, quat.z);
        }

        Timer::after_millis(10).await;
    }
}

