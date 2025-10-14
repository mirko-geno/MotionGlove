#![no_std]
#![no_main]

use embassy_time::Duration;
use embassy_rp::{
    gpio::{Input, Level},
    adc::{self, Adc},
};
use usbd_hid::descriptor::{
    MouseReport,
    KeyboardReport,
    MediaKeyboardReport,
};

pub mod sensors;
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

pub type HidInstructionArr = [u8;16];

pub struct FingerFlexes<'a> {
    adc_driver: Adc<'a, adc::Async>,
    thumb_flex: adc::Channel<'a>,
    index_flex: adc::Channel<'a>,
    middle_flex: adc::Channel<'a>,
}

impl<'a> FingerFlexes<'a> {
    pub fn new(adc_driver: Adc<'a, adc::Async>, thumb_flex: adc::Channel<'a>, index_flex: adc::Channel<'a>, middle_flex: adc::Channel<'a>,) -> Self{
        FingerFlexes {adc_driver, thumb_flex, index_flex, middle_flex}
    }

    pub async fn read(&mut self) -> [u16;3] {
        let thumb   = self.adc_driver.read(&mut self.thumb_flex).await.unwrap();
        let index   = self.adc_driver.read(&mut self.index_flex).await.unwrap();
        let middle  = self.adc_driver.read(&mut self.middle_flex).await.unwrap();
        [thumb, index, middle]
    }
}

pub struct HidInstruction {
    pub mouse: MouseReport,
    pub keyboard: KeyboardReport,
    pub media: MediaKeyboardReport,
}

impl HidInstruction {
    /// Build HidInstruction from big endian bytes [u8;16]
    pub fn from_be_bytes(data: HidInstructionArr) -> Self {
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

    /// Converts HidInstruction to big endian bytes [u8;16]
    pub fn to_be_bytes(&self) -> HidInstructionArr {
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

