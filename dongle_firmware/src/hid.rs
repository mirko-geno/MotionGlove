use embassy_rp::{
    peripherals::USB,
    usb::Driver,
};
use embassy_usb::{
    class::hid::{self, HidReaderWriter},
    UsbDevice
};
use embassy_time::Timer;
use usbd_hid::descriptor::{MouseReport, KeyboardReport, SerializedDescriptor};
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};


// USB Descriptors
static CONFIG_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
static BOS_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
static MSOS_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
static CONTROL_BUF: StaticCell<[u8; 64]> = StaticCell::new();


pub fn config_usb(driver: Driver<'static, USB>
) -> (
UsbDevice<'static, embassy_rp::usb::Driver<'static, USB>>,
HidReaderWriter<'static, embassy_rp::usb::Driver<'static, USB>, 1, 8>,
HidReaderWriter<'static, embassy_rp::usb::Driver<'static, USB>, 1, 8>
) {
    // Create embassy-usb Config
    let mut config  = embassy_usb::Config::new(0xc0de, 0xcafe);
    config.manufacturer = Some("LosDos");
    config.product = Some("MotionGlove");
    config.serial_number = Some("22222222");
    config.max_power = 100;
    config.max_packet_size_0 = 64;

    // Init static memory
    let config_descriptor = CONFIG_DESCRIPTOR.init([0; 256]);
    let bos_descriptor = BOS_DESCRIPTOR.init([0; 256]);
    let msos_descriptor = MSOS_DESCRIPTOR.init([0; 256]);
    let control_buf = CONTROL_BUF.init([0; 64]);

    // USB Builder
    let mut builder = embassy_usb::Builder::new(
        driver,
        config,
        config_descriptor,
        bos_descriptor,
        msos_descriptor,
        control_buf,
    );

    // Mouse config
    static MOUSE_STATE: StaticCell<hid::State> = StaticCell::new();
    let mouse_state = MOUSE_STATE.init(hid::State::new());
    let mouse_config = embassy_usb::class::hid::Config {
        report_descriptor: MouseReport::desc(),
        request_handler: None,
        poll_ms: 60,
        max_packet_size: 64,
    };
    let hid_mouse = HidReaderWriter::<_, 1, 8>::new(&mut builder, mouse_state, mouse_config);

    // Keyboard config
    static KEYBOARD_STATE: StaticCell<hid::State> = StaticCell::new();
    let keyboard_state = KEYBOARD_STATE.init(hid::State::new());
    let keyboard_config = embassy_usb::class::hid::Config {
        report_descriptor: KeyboardReport::desc(),
        request_handler: None,
        poll_ms: 10,
        max_packet_size: 64,
    };
    let hid_keyboard = HidReaderWriter::<_, 1, 8>::new(&mut builder, keyboard_state, keyboard_config);

    // USB Build
    let usb = builder.build();

    (usb, hid_mouse, hid_keyboard)
}


#[embassy_executor::task]
pub async fn hid_usb(
mut hid_mouse: HidReaderWriter<'static, embassy_rp::usb::Driver<'static, USB>, 1, 8>,
mut hid_keyboard: HidReaderWriter<'static, embassy_rp::usb::Driver<'static, USB>, 1, 8> 
) -> ! {
    loop {
        let mouse_report = MouseReport {
            buttons: 0,
            x: 5,
            y: 3,
            wheel: 0,
            pan: 0,
        };
        let keyboard_report = KeyboardReport {
            modifier: 0,
            reserved: 0,
            leds: 0,
            keycodes: [4, 0, 0, 0, 0, 0], // 'a'
        };

        if let Err(e) = hid_mouse.write_serialize(&mouse_report).await {
            log::warn!("Failed to send mouse report: {:?}", e);
        }
        if let Err(e) = hid_keyboard.write_serialize(&keyboard_report).await {
            log::warn!("Failed to send keyboard report: {:?}", e);
        }
        Timer::after_secs(1).await
    }
}
