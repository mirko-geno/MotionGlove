#![no_std]
#![no_main]

use defmt::*;
use embassy_time::Delay;
use {defmt_rtt as _, panic_probe as _};
use embassy_executor::Spawner;
use embassy_rp::{
    bind_interrupts,
    gpio::{Input, Output, Pull, Level},
    peripherals::{DMA_CH0, PIO0, I2C0, USB},
    pio::{self, Pio}, 
    i2c::{self, I2c},
    usb::{self, Driver},
    adc::{self, Adc, Config as AdcConfig}
};
use embassy_sync::{
    channel::Channel,
    blocking_mutex::raw::CriticalSectionRawMutex,
};
use cyw43_pio::{PioSpi, DEFAULT_CLOCK_DIVIDER};

use static_cell::StaticCell;

use mpu9250_async::{address::Address, sensor_async::Mpu9250};

use shared::{
    definitions::CHANNEL_SIZE,
    custom_hid::HidInstruction,
};

use glove::{
    // blinker::blink_task,
    sensors::sensor_processing,
    flexes::FingerFlexes,
    tcp_client::{network_config, tcp_client_task},
};


bind_interrupts!(
    struct Irqs {
        PIO0_IRQ_0 => pio::InterruptHandler<PIO0>;
        I2C0_IRQ => i2c::InterruptHandler<I2C0>;
        USBCTRL_IRQ => usb::InterruptHandler<USB>;
        ADC_IRQ_FIFO => adc::InterruptHandler;
    }
);


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
    // Pico init and pin configuration
    let p = embassy_rp::init(Default::default());
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

    // Set usb_logger
    let driver = Driver::new(p.USB, Irqs);
    unwrap!(spawner.spawn(logger_task(driver)));
    
    // cyw43 wifi chip init
    let fw = include_bytes!("../../shared/cyw43-firmware/43439A0.bin");
    let clm = include_bytes!("../../shared/cyw43-firmware/43439A0_clm.bin");
    
    static STATE: StaticCell<cyw43::State> = StaticCell::new();
    let state = STATE.init(cyw43::State::new());
    let (net_device, mut control, runner) = cyw43::new(state, pwr, spi, fw).await;
    
    unwrap!(spawner.spawn(cyw43_task(runner)));
    control.init(clm).await;
    control
        .set_power_management(cyw43::PowerManagementMode::None) // Use cyw43::PowerManagementMode::None if too much latency
        .await;

    // unwrap!(spawner.spawn(blink_task(control)));

    let (stack, runner) = network_config(net_device);
    unwrap!(spawner.spawn(net_task(runner)));

    static CHANNEL: Channel<CriticalSectionRawMutex, HidInstruction, CHANNEL_SIZE> = Channel::new();
    let tx_ch = CHANNEL.sender();
    let rx_ch = CHANNEL.receiver();

    unwrap!(spawner.spawn(tcp_client_task(control, stack, rx_ch)));

    // Instantiate ADC flex sensors
    let adc_driver   = Adc::new(p.ADC, Irqs, AdcConfig::default());
    let thumb_flex      = adc::Channel::new_pin(p.PIN_28, Pull::Down);
    let index_flex      = adc::Channel::new_pin(p.PIN_27, Pull::Down);
    let middle_flex     = adc::Channel::new_pin(p.PIN_26, Pull::Down);

    let finger_flexes = FingerFlexes::new(adc_driver, thumb_flex, index_flex, middle_flex);

    // Instantiate Finger tap
    let mut finger_tap = Input::new(p.PIN_22, Pull::Down);
    finger_tap.set_schmitt(true);

    // Instantiate mpu sensor
    let sda = p.PIN_4; // GP20, PIN26
    let scl = p.PIN_5; // GP21, PIN27

    let i2c_config = i2c::Config::default();
    let i2c_bus = I2c::new_async(p.I2C0, scl, sda, Irqs, i2c_config);
    let mpu = Mpu9250::new(i2c_bus, Address::default(), &mut Delay).await.unwrap();
    unwrap!(spawner.spawn(sensor_processing(mpu, finger_flexes, finger_tap, tx_ch)));
}
