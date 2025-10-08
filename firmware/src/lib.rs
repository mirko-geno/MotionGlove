#![no_std]
#![no_main]

use embassy_time::Duration;
use mpu6050_dmp::{
    accel::Accel,
    gyro::Gyro
};

pub mod mpu;
pub mod blinker;
pub mod tcp_client;

pub const WIFI_NETWORK: &str = "MotionGloveConnection";
pub const WIFI_PASSWORD: &str = "MGlove2025";
pub const TCP_CHANNEL: u8 = 5;
pub const TCP_ENDPOINT: u16 = 50124;
pub const SOCKET_TIMEOUT: Duration = Duration::from_secs(15);
pub const DONGLE_IP: &str = "169.254.1.1";
pub const SENDER_IP: &str = "169.254.1.2";
pub const CHANNEL_SIZE: usize = 1;
pub const READ_FREQ: u64 = 1000;
pub type MessageArr = [u8;12];

#[derive(Debug)]
pub struct SensorReadings {
    pub accel: Accel,
    pub gyro: Gyro,
}

impl SensorReadings {
    /// Builds a message from Accelerometer and Gyroscope readings
    /// Accel (x, y, z)
    /// Gyro (x, y, z)
    pub fn from(accel: Accel, gyro: Gyro) -> Self {
        SensorReadings { accel, gyro }
    }

    /// Builds Message type from [u8;12] Array
    pub const fn from_bytes(data: [u8; 12]) -> Self {
        let accel   = [data[0], data[1], data[2], data[3], data[4], data[5]];
        let gyro    = [data[6], data[7], data[8], data[9], data[10], data[11]];

        Self {
            accel: Accel::from_bytes(accel),
            gyro: Gyro::from_bytes(gyro),
        }
    }
    
    /// Returns full message as [u8;12]
    pub const fn as_bytes(&self) -> [u8; 12] {
        let accel_x = self.accel.x().to_be_bytes();
        let accel_y = self.accel.y().to_be_bytes();
        let accel_z = self.accel.z().to_be_bytes();
        let gyro_x  = self.gyro.x().to_be_bytes();
        let gyro_y  = self.gyro.y().to_be_bytes();
        let gyro_z  = self.gyro.z().to_be_bytes();
        [
            accel_x[0], accel_x[1], accel_y[0], accel_y[1], accel_z[0], accel_z[1],
            gyro_x[0], gyro_x[1], gyro_y[0], gyro_y[1], gyro_z[0], gyro_z[1],
        ]
    }
}
