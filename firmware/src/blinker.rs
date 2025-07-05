use {defmt_rtt as _, panic_probe as _};
use embassy_time::{Timer, Duration};


#[embassy_executor::task]
pub async fn blink_task(mut control: cyw43::Control<'static>) -> ! {
    let delay = Duration::from_secs(1);
    loop {
        control.gpio_set(0, true).await;
        Timer::after(delay).await;

        control.gpio_set(0, false).await;
        Timer::after(delay).await;
    }
}