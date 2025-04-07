use {defmt_rtt as _, panic_probe as _};
use embassy_time::{Duration, Timer};
use embassy_rp::{
    peripherals::I2C0, 
    i2c::{self, I2c},
};
use mpu6050_async::*;


#[embassy_executor::task]
pub async fn read_mpu(mut mpu: Mpu6050<I2c<'static, I2C0, i2c::Async>>) -> ! {
    loop {
        // get roll and pitch estimate
        let acc = mpu.get_acc_angles().await.unwrap();
        log::info!("r/p: {:?}", acc);

        // get temp
        let temp = mpu.get_temp().await.unwrap();
        log::info!("temp: {:?}c", temp);

        // get gyro data, scaled with sensitivity
        let gyro = mpu.get_gyro().await.unwrap();
        log::info!("gyro: {:?}", gyro);

        // get accelerometer data, scaled with sensitivity
        let acc = mpu.get_acc().await.unwrap();
        log::info!("acc: {:?}", acc);

        Timer::after(Duration::from_secs(1)).await;
    }
}
