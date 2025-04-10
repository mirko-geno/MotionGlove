use {defmt_rtt as _, panic_probe as _};
use embassy_time::{Duration, Timer};
use embassy_rp::{
    peripherals::I2C0, 
    i2c::{self, I2c},
};
use mpu6050_async::*;


async fn calibrate_gyro(mpu: &mut Mpu6050<I2c<'static, I2C0, i2c::Async>>) -> (f32, f32, f32) {
    let delay = Duration::from_millis(10);
    let cali_params = 1000.0;
    let mut x_reg: f32 = 0.0; 
    let mut y_reg: f32 = 0.0;
    let mut z_reg: f32 = 0.0;

    for _ in 1..(cali_params as i32) {
        let (x, y, z) = mpu.get_gyro().await.unwrap();
        x_reg += x;
        y_reg += y;
        z_reg += z;
        Timer::after(delay).await;
    }

    (x_reg/cali_params, y_reg/cali_params, z_reg/cali_params)
}


#[embassy_executor::task]
pub async fn read_mpu(mut mpu: Mpu6050<I2c<'static, I2C0, i2c::Async>>) -> ! {
    let cali_error = calibrate_gyro(&mut mpu).await;
    loop {
        // get roll and pitch estimate
        let acc = mpu.get_acc_angles().await.unwrap();
        log::info!("roll/pitch: {:?}", acc);

        // get gyro data, scaled with sensitivity
        let gyro = mpu.get_gyro().await.unwrap();
        let calibrated_gyro = (gyro.0 - cali_error.0, gyro.1 - cali_error.1, gyro.2 - cali_error.2);
        log::info!("gyro: {:?}", calibrated_gyro);

        // get accelerometer data, scaled with sensitivity
        let acc = mpu.get_acc().await.unwrap();
        log::info!("acc: {:?}", acc);

        Timer::after(Duration::from_millis(10)).await;
    }
}
