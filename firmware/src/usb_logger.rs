
use {defmt_rtt as _, panic_probe as _};
use embassy_rp::{
    peripherals::USB,
    usb::Driver,
};


#[embassy_executor::task]
pub async fn logger_task(driver: Driver<'static, USB>) {
    embassy_usb_logger::run!(1024, log::LevelFilter::Info, driver);
}