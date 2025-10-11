#![no_std]
#![no_main]

use embassy_time::Duration;
use mpu6050_dmp::{
    accel::Accel,
    gyro::Gyro, quaternion
};
use usbd_hid::descriptor::{
    MouseReport,
    KeyboardReport, KeyboardUsage,
    MediaKeyboardReport, MediaKey,
    SerializedDescriptor
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

pub struct HidInstruction {
    mouse: MouseReport,
    keyboard: KeyboardReport,
    media: MediaKeyboardReport,
}

impl HidInstruction {
    /// Build HidInstruction from Reports (Mouse, Keyboard and Media)
    pub fn from(mouse: MouseReport, keyboard: KeyboardReport, media: MediaKeyboardReport) -> Self {
        HidInstruction { mouse, keyboard, media }
    }

    /// Build HidInstruction from big endian bytes
    pub fn from_be_bytes(data: [u8;16]) -> Self {
        let mouse = MouseReport {
            buttons:    u8::from_be(data[0]),
            x:          u8::from_be(data[1]) as i8,
            y:          u8::from_be(data[2]) as i8,
            wheel:      u8::from_be(data[3]) as i8,
            pan:        u8::from_be(data[4]) as i8
        };
        let keyboard = KeyboardReport {
            modifier:   u8::from_be(data[5]),
            reserved:   u8::from_be(data[6]),
            leds:       u8::from_be(data[7]),
            keycodes:   [data[8], data[9], data[10], data[11], data[12], data[13]]
        };
        let media = MediaKeyboardReport {
            usage_id:   u16::from_be_bytes([data[14], data[15]])
        };

        HidInstruction { mouse, keyboard, media }
    }

    /// Converts HidInstruction to big endian bytes
    pub fn to_be_bytes(&self) -> [u8; 16] {
        let mouse_buttons            = self.mouse.buttons.to_be();
        let mouse_x                  = (self.mouse.x as u8).to_be();
        let mouse_y                  = (self.mouse.y as u8).to_be();
        let mouse_wheel              = (self.mouse.wheel as u8).to_be();
        let mouse_pan                = (self.mouse.pan as u8).to_be();
        
        let keyboard_modifier        = self.keyboard.modifier.to_be();
        let keyboard_reserved        = self.keyboard.reserved.to_be();
        let keyboard_leds            = self.keyboard.leds.to_be();
        let keyboard_keycode    = self.keyboard.keycodes;

        let media_usage_id      = self.media.usage_id.to_be_bytes();

        [
            mouse_buttons, mouse_x, mouse_y, mouse_wheel, mouse_pan,
            keyboard_modifier, keyboard_reserved, keyboard_leds,
            keyboard_keycode[0], keyboard_keycode[1], keyboard_keycode[2], 
            keyboard_keycode[3], keyboard_keycode[4], keyboard_keycode[5],
            media_usage_id[0], media_usage_id[1]
        ]
    }
}


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
