#![no_std]
#![no_main]

use defmt::*;
use {defmt_rtt as _, panic_probe as _};
use embassy_executor::Spawner;
use embassy_time::{Delay, Duration, Timer};
use embassy_rp::{
    bind_interrupts,
    gpio::{Level, Output},
    peripherals::{DMA_CH0, PIO0, I2C0, USB},
    pio::{self, Pio}, 
    i2c::{self, I2c},
    usb::{self, Driver},
};
use cyw43_pio::{PioSpi, DEFAULT_CLOCK_DIVIDER};

use static_cell::StaticCell;

use mpu6050_async::*;

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => pio::InterruptHandler<PIO0>;
    I2C0_IRQ => i2c::InterruptHandler<I2C0>;
    USBCTRL_IRQ => usb::InterruptHandler<USB>;
});

const DELAY: embassy_time::Duration = Duration::from_secs(1);


#[embassy_executor::task]
async fn cyw43_task(runner: cyw43::Runner<'static, Output<'static>, PioSpi<'static, PIO0, 0, DMA_CH0>>) -> ! {
    runner.run().await
}


#[embassy_executor::task]
async fn logger_task(driver: Driver<'static, USB>) {
    embassy_usb_logger::run!(1024, log::LevelFilter::Info, driver);
}


#[embassy_executor::task]
async fn read_mpu(mut mpu: Mpu6050<I2c<'static, I2C0, i2c::Async>>) -> ! {
    loop {
        // get roll and pitch estimate
        let acc = mpu.get_acc_angles().await.unwrap();
        log::info!("r/p: {:?}", acc);

        // get temp
        let temp = mpu.get_temp().await.unwrap();
        log::info!("temp: {:?}c", temp);

        // get gyro data, scaled with sensitivity
        let gyro = mpu.get_gyro().await.unwrap();
        log::info!("gyro: {:?}", gyro);

        // get accelerometer data, scaled with sensitivity
        let acc = mpu.get_acc().await.unwrap();
        log::info!("acc: {:?}", acc);

        Timer::after(Duration::from_secs(1)).await;
    }
}

#[embassy_executor::task]
async fn blink_task(mut control: cyw43::Control<'static>) -> ! {
    loop {
        control.gpio_set(0, true).await;
        Timer::after(DELAY).await;

        control.gpio_set(0, false).await;
        Timer::after(DELAY).await;
    }
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
    let sda = p.PIN_20; // GP20, PIN26
    let scl = p.PIN_21; // GP21, PIN27
    let config = i2c::Config::default();
    let bus = I2c::new_async(p.I2C0, scl, sda, Irqs, config);

    let mut mpu = Mpu6050::new(bus);
    mpu.init(&mut Delay).await.unwrap();

    static STATE: StaticCell<cyw43::State> = StaticCell::new();
    let state = STATE.init(cyw43::State::new());
    let (_net_device, mut control, runner) = cyw43::new(state, pwr, spi, fw).await;
    unwrap!(spawner.spawn(cyw43_task(runner)));

    control.init(clm).await;
    control
        .set_power_management(cyw43::PowerManagementMode::PowerSave) // Use cyw43::PowerManagementMode::Disabled if too much latency
        .await;
    
    unwrap!(spawner.spawn(logger_task(driver)));
    unwrap!(spawner.spawn(blink_task(control)));
    unwrap!(spawner.spawn(read_mpu(mpu)));
}
