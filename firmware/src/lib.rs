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


pub struct Message {
    content: &'static [u8]
}

impl Message {
    pub fn new(content: &'static [u8]) -> Self {
        Self { content: content }
    }

    pub fn to_send(&mut self) -> &'static [u8] {
        self.content
    }
}