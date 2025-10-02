use core::sync::atomic::{AtomicBool, Ordering};
use defmt::*;
use embassy_rp::{
    peripherals::USB,
    usb::Driver,
};
use embassy_usb::{
    class::hid::{self, HidReaderWriter, ReportId, RequestHandler},
    control::OutResponse,
    Handler,
};
use embassy_futures::join::join;
use embassy_time::Timer;
use usbd_hid::descriptor::{MouseReport, KeyboardReport, SerializedDescriptor};
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};


// USB Descriptors
static CONFIG_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
static BOS_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
static MSOS_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
static CONTROL_BUF: StaticCell<[u8; 64]> = StaticCell::new();

// USB Handlers
static DEVICE_HANDLER: StaticCell<MyDeviceHandler> = StaticCell::new();


struct MyRequestHandler {}

impl RequestHandler for MyRequestHandler {
    fn get_report(&mut self, id: ReportId, _buf: &mut [u8]) -> Option<usize> {
        info!("Get report for {:?}", id);
        None
    }

    fn set_report(&mut self, id: ReportId, data: &[u8]) -> OutResponse {
        info!("Set report for {:?}: {=[u8]}", id, data);
        OutResponse::Accepted
    }

    fn set_idle_ms(&mut self, id: Option<ReportId>, dur: u32) {
        info!("Set idle rate for {:?} to {:?}", id, dur);
    }

    fn get_idle_ms(&mut self, id: Option<ReportId>) -> Option<u32> {
        info!("Get idle rate for {:?}", id);
        None
    }
}

struct MyDeviceHandler {
    configured: AtomicBool,
}

impl MyDeviceHandler {
    fn new() -> Self {
        MyDeviceHandler {
            configured: AtomicBool::new(false),
        }
    }
}

impl Handler for MyDeviceHandler {
    fn enabled(&mut self, enabled: bool) {
        self.configured.store(false, Ordering::Relaxed);
        if enabled {
            info!("Device enabled");
        } else {
            info!("Device disabled");
        }
    }

    fn reset(&mut self) {
        self.configured.store(false, Ordering::Relaxed);
        info!("Bus reset, the Vbus current limit is 100mA");
    }

    fn addressed(&mut self, addr: u8) {
        self.configured.store(false, Ordering::Relaxed);
        info!("USB address set to: {}", addr);
    }

    fn configured(&mut self, configured: bool) {
        self.configured.store(configured, Ordering::Relaxed);
        if configured {
            info!("Device configured, it may now draw up to the configured current limit from Vbus.")
        } else {
            info!("Device is no longer configured, the Vbus current limit is 100mA.");
        }
    }
}


#[embassy_executor::task]
pub async fn hid_usb(driver: Driver<'static, USB>) -> () {
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
    let device_handler = DEVICE_HANDLER.init(MyDeviceHandler::new());

    // USB Builder
    let mut builder = embassy_usb::Builder::new(
        driver,
        config,
        config_descriptor,
        bos_descriptor,
        msos_descriptor,
        control_buf,
    );
    builder.handler(device_handler);

    // Mouse config
    static MOUSE_STATE: StaticCell<hid::State> = StaticCell::new();
    let mouse_state = MOUSE_STATE.init(hid::State::new());
    let mouse_config = embassy_usb::class::hid::Config {
        report_descriptor: MouseReport::desc(),
        request_handler: None,
        poll_ms: 60,
        max_packet_size: 64,
    };
    let mouse_hid = HidReaderWriter::<_, 1, 8>::new(&mut builder, mouse_state, mouse_config);

    // Keyboard config
    static KEYBOARD_STATE: StaticCell<hid::State> = StaticCell::new();
    let keyboard_state = KEYBOARD_STATE.init(hid::State::new());
    let keyboard_config = embassy_usb::class::hid::Config {
        report_descriptor: KeyboardReport::desc(),
        request_handler: None,
        poll_ms: 10,
        max_packet_size: 64,
    };
    let keyboard_hid = HidReaderWriter::<_, 1, 8>::new(&mut builder, keyboard_state, keyboard_config);

    // USB Build
    let mut usb = builder.build();
    let usb_fut = usb.run();


    let (_mouse_reader, mut mouse_writer) = mouse_hid.split();
    let (_kbd_reader, mut kbd_writer) = keyboard_hid.split();

    // Mouse reports
    let mouse_fut = async {
        loop {
            _ = Timer::after_secs(1).await;
            let report = MouseReport {
                buttons: 0,
                x: 5,
                y: 3,
                wheel: 0,
                pan: 0,
            };
            if let Err(e) = mouse_writer.write_serialize(&report).await {
                warn!("Failed to send mouse report: {:?}", e);
            }
        }
    };

    // Keyboard reports
    let keyboard_fut = async {
        loop {
            _ = Timer::after_secs(2).await;
            let report = KeyboardReport {
                modifier: 0,
                reserved: 0,
                leds: 0,
                keycodes: [4, 0, 0, 0, 0, 0], // tecla 'a'
            };
            if let Err(e) = kbd_writer.write_serialize(&report).await {
                warn!("Failed to send keyboard report: {:?}", e);
            }
        }
    };

    // Execute everything concurrently
    join(usb_fut, join(mouse_fut, keyboard_fut)).await;
}
