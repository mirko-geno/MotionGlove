use embassy_rp::{
    peripherals::USB,
    usb::Driver,
};
use embassy_usb::{
    class::{
        hid::{HidReaderWriter, Config as HidConfig, State as HidState},
        cdc_acm::{CdcAcmClass, State as CdcState},
    },
    UsbDevice
};
use embassy_usb_logger::MAX_PACKET_SIZE;
use embassy_sync::{
    channel::Receiver,
    blocking_mutex::raw::CriticalSectionRawMutex,
};
use usbd_hid::descriptor::{MouseReport, KeyboardReport, MediaKeyboardReport, MediaKey, SerializedDescriptor};
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};

use shared::{
    definitions::{
        CHANNEL_SIZE,
        MOUSE_POLL_MS
    },
    custom_hid::HidInstruction
};

// USB Descriptors
static CONFIG_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
static BOS_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
static MSOS_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
static CONTROL_BUF: StaticCell<[u8; 64]> = StaticCell::new();

type HidDevice = HidReaderWriter<'static, Driver<'static, USB>, 1, 8>;
type LoggerDevice = CdcAcmClass<'static, Driver<'static, USB>>;

pub fn config_usb(driver: Driver<'static, USB>) -> (UsbDevice<'static, Driver<'static, USB>>, LoggerDevice, HidDevice, HidDevice, HidDevice) {
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

    // USB Logger config
    static USB_LOGGER: StaticCell<CdcState<'static>> = StaticCell::new();
    let logger_state = USB_LOGGER.init(CdcState::new());
    let logger = CdcAcmClass::new(&mut builder, logger_state, MAX_PACKET_SIZE.into());

    // Mouse config
    static MOUSE_STATE: StaticCell<HidState> = StaticCell::new();
    let mouse_state = MOUSE_STATE.init(HidState::new());
    let mouse_config = HidConfig {
        report_descriptor: MouseReport::desc(),
        request_handler: None,
        poll_ms: MOUSE_POLL_MS,
        max_packet_size: 64,
    };
    let hid_mouse = HidReaderWriter::<_, 1, 8>::new(&mut builder, mouse_state, mouse_config);

    // Keyboard config
    static KEYBOARD_STATE: StaticCell<HidState> = StaticCell::new();
    let keyboard_state = KEYBOARD_STATE.init(HidState::new());
    let keyboard_config = HidConfig {
        report_descriptor: KeyboardReport::desc(),
        request_handler: None,
        poll_ms: MOUSE_POLL_MS,
        max_packet_size: 64,
    };
    let hid_keyboard = HidReaderWriter::<_, 1, 8>::new(&mut builder, keyboard_state, keyboard_config);

    // MediaKeyboard config
    static MEDIA_KEYBOARD_STATE: StaticCell<HidState> = StaticCell::new();
    let media_state = MEDIA_KEYBOARD_STATE.init(HidState::new());
    let media_config = HidConfig {
        report_descriptor: MediaKeyboardReport::desc(),
        request_handler: None,
        poll_ms: MOUSE_POLL_MS,
        max_packet_size: 64,
    };
    let hid_media = HidReaderWriter::<_, 1, 8>::new(&mut builder, media_state, media_config);

    // USB Build
    let usb = builder.build();

    (usb, logger, hid_mouse, hid_keyboard, hid_media)
}


#[embassy_executor::task]
pub async fn hid_usb_controller(mut hid_mouse: HidDevice, mut hid_keyboard: HidDevice, mut hid_media: HidDevice,
rx_ch: Receiver<'static, CriticalSectionRawMutex, HidInstruction, CHANNEL_SIZE>) -> ! {
    loop {
        let hid_instruction = rx_ch.receive().await;
        let release_keyboard = KeyboardReport {
            modifier: 0,
            reserved: 0,
            leds: 0,
            keycodes: [0, 0, 0, 0, 0, 0], // None
        };
        let release_media = MediaKeyboardReport {
            usage_id: MediaKey::Zero.into()
        };

        if let Err(e) = hid_mouse.write_serialize(&hid_instruction.mouse).await {
            log::warn!("Failed to send mouse report: {:?}", e);
        }
        if let Err(e) = hid_keyboard.write_serialize(&hid_instruction.keyboard).await {
            log::warn!("Failed to send keyboard report: {:?}", e);
        }
        if let Err(e) = hid_keyboard.write_serialize(&release_keyboard).await {
            log::warn!("Failed to release keyboard report: {:?}", e);
        }
        if let Err(e) = hid_media.write_serialize(&hid_instruction.media).await {
            log::warn!("Failed to send keyboard report: {:?}", e);
        }
        if let Err(e) = hid_media.write_serialize(&release_media).await {
            log::warn!("Failed to release keyboard report: {:?}", e);
        }
    }
}
