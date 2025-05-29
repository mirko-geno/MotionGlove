#![no_std]
#![no_main]

use defmt::*;
use {defmt_rtt as _, panic_probe as _};
use embassy_executor::Spawner;
use embassy_rp::{
    bind_interrupts,
    gpio::{Level, Output},
    peripherals::{DMA_CH0, PIO0, I2C1, USB},
    pio::{self, Pio}, 
    i2c::{self, I2c},
    usb::{self, Driver},
};
use cyw43_pio::{PioSpi, DEFAULT_CLOCK_DIVIDER};

use static_cell::StaticCell;

use mpu6050_dmp::{address::Address, sensor_async::Mpu6050};

use mape_2025::{
    usb_logger::logger_task,
    blinker::blink_task,
    mpu::read_mpu,
};

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => pio::InterruptHandler<PIO0>;
    I2C1_IRQ => i2c::InterruptHandler<I2C1>;
    USBCTRL_IRQ => usb::InterruptHandler<USB>;
});


#[embassy_executor::task]
async fn cyw43_task(runner: cyw43::Runner<'static, Output<'static>, PioSpi<'static, PIO0, 0, DMA_CH0>>) -> ! {
    runner.run().await
}


#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let fw = include_bytes!("../cyw43-firmware/43439A0.bin");
    let clm = include_bytes!("../cyw43-firmware/43439A0_clm.bin");

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
    let driver = Driver::new(p.USB, Irqs);
    let sda = p.PIN_26; // GP20, PIN26
    let scl = p.PIN_27; // GP21, PIN27
    let config = i2c::Config::default();
    let bus = I2c::new_async(p.I2C1, scl, sda, Irqs, config);

    let mpu = Mpu6050::new(bus, Address::default()).await.unwrap();

    static STATE: StaticCell<cyw43::State> = StaticCell::new();
    let state = STATE.init(cyw43::State::new());
    let (_net_device, mut control, runner) = cyw43::new(state, pwr, spi, fw).await;
    unwrap!(spawner.spawn(cyw43_task(runner)));

    control.init(clm).await;
    control
        .set_power_management(cyw43::PowerManagementMode::PowerSave) // Use cyw43::PowerManagementMode::None if too much latency
        .await;
    
    unwrap!(spawner.spawn(logger_task(driver)));
    unwrap!(spawner.spawn(blink_task(control)));
    unwrap!(spawner.spawn(read_mpu(mpu)));
    
}
