#![no_std]
#![no_main]
#![allow(async_fn_in_trait)]

use defmt::*;
use cyw43_pio::{PioSpi, DEFAULT_CLOCK_DIVIDER};
use embassy_executor::Spawner;
use embassy_rp::{
    bind_interrupts,
    gpio::{Level, Output},
    peripherals::{DMA_CH0, PIO0, USB},
    pio::{InterruptHandler, Pio},
    usb::{self, Driver},
    clocks::RoscRng
};
use embassy_net::{
    StackResources,
    Config
};
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};

use dongle_firmware::tcp_server::tcp_server_task;

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => InterruptHandler<PIO0>;
    USBCTRL_IRQ => usb::InterruptHandler<USB>;
});


#[embassy_executor::task]
async fn cyw43_task(runner: cyw43::Runner<'static, Output<'static>, PioSpi<'static, PIO0, 0, DMA_CH0>>) -> ! {
    runner.run().await
}

#[embassy_executor::task]
async fn net_task(mut runner: embassy_net::Runner<'static, cyw43::NetDriver<'static>>) -> ! {
    runner.run().await
}

#[embassy_executor::task]
pub async fn logger_task(driver: Driver<'static, USB>) {
    embassy_usb_logger::run!(1024, log::LevelFilter::Info, driver);
}


#[embassy_executor::main]
async fn main(spawner: Spawner) {
    // Pico init
    let p = embassy_rp::init(Default::default());

    // Set usb_logger
    let driver = Driver::new(p.USB, Irqs);
    unwrap!(spawner.spawn(logger_task(driver)));

    // cyw43 wifi chip init
    let fw = include_bytes!("../../firmware/cyw43-firmware/43439A0.bin");
    let clm = include_bytes!("../../firmware/cyw43-firmware/43439A0_clm.bin");

    let pwr = Output::new(p.PIN_23, Level::Low);
    let cs = Output::new(p.PIN_25, Level::High);
    let mut pio = Pio::new(p.PIO0, Irqs);
    let spi = PioSpi::new(
        &mut pio.common,
        pio.sm0,
        DEFAULT_CLOCK_DIVIDER,
        pio.irq0,
        cs,
        p.PIN_24,
        p.PIN_29,
        p.DMA_CH0,
    );

    static STATE: StaticCell<cyw43::State> = StaticCell::new();
    let state = STATE.init(cyw43::State::new());
    let (net_device, mut control, runner) = cyw43::new(state, pwr, spi, fw).await;
    unwrap!(spawner.spawn(cyw43_task(runner)));

    control.init(clm).await;
    control
        .set_power_management(cyw43::PowerManagementMode::PowerSave)
        .await;

    // Configure the network
    // Use a link-local address for communication without DHCP server
    let config = Config::dhcpv4(Default::default());

    // Generate random seed
    let seed = RoscRng.next_u64();

    // Init network stack
    static RESOURCES: StaticCell<StackResources<3>> = StaticCell::new();
    let (stack, runner) = embassy_net::new(net_device, config, RESOURCES.init(StackResources::new()), seed);
    unwrap!(spawner.spawn(net_task(runner)));

    unwrap!(spawner.spawn(tcp_server_task(control, stack)));
}