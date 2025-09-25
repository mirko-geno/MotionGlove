#![no_std]
#![no_main]

pub mod mpu;
pub mod blinker;
pub mod tcp_client;

pub const WIFI_NETWORK: &str = "MotionGloveConnection";
pub const WIFI_PASSWORD: &str = "MGlove2025";
pub const TCP_CHANNEL: u8 = 5;
pub const TCP_ENDPOINT: u16 = 50124;
pub const DONGLE_IP: &str = "169.254.1.1";
pub const SENDER_IP: &str = "169.254.1.2";
pub const MESSAGE_LENGTH: usize = 64;
pub const CHANNEL_SIZE: usize = 2;
