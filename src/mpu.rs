use {defmt_rtt as _, panic_probe as _};
use embassy_time::{Duration, Timer};
use embassy_rp::{
    peripherals::I2C0, 
    i2c::{self, I2c},
};
use mpu6050_async::*;


async fn _calibrate_gyro(mpu: &mut Mpu6050<I2c<'static, I2C0, i2c::Async>>) -> (f64, f64, f64) {
    let mut x_reg: [f32; 100];
    let mut y_reg: [f32; 100];
    let mut z_reg: [f32; 100];

    let (x, y, z) = mpu.get_gyro().await.unwrap();
    x_reg.concat(x);

    
    


    return (1.0, 2.1, 3.2);
}


#[embassy_executor::task]
pub async fn read_mpu(mut mpu: Mpu6050<I2c<'static, I2C0, i2c::Async>>) -> ! {
    loop {
        // get roll and pitch estimate
        let acc = mpu.get_acc_angles().await.unwrap();
        log::info!("roll/pitch: {:?}", acc);

        // get gyro data, scaled with sensitivity
        let gyro = mpu.get_gyro().await.unwrap();
        log::info!("gyro: {:?}", gyro);

        // get accelerometer data, scaled with sensitivity
        let acc = mpu.get_acc().await.unwrap();
        log::info!("acc: {:?}", acc);

        Timer::after(Duration::from_millis(10)).await;
    }
}
