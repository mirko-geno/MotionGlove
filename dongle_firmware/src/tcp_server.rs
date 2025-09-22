use {defmt_rtt as _, panic_probe as _};

use core::str::from_utf8;
use cyw43::JoinOptions;
use embassy_net::{
    Stack,
    tcp::TcpSocket,
};
use embassy_time::Duration;
use embedded_io_async::Write;

const WIFI_NETWORK: &str = "MirkoWifi"; // change to your network SSID
const WIFI_PASSWORD: &str = "password123"; // change to your network password

#[embassy_executor::task]
pub async fn tcp_server_task(mut control: cyw43::Control<'static>, stack: Stack<'static>) -> ! {
    // Try connection wifi
    while let Err(err) = control
        .join(WIFI_NETWORK, JoinOptions::new(WIFI_PASSWORD.as_bytes()))
        .await
    {
        log::info!("join failed with status={}", err.status);
    }

    log::info!("waiting for link...");
    stack.wait_link_up().await;

    log::info!("waiting for DHCP...");
    stack.wait_config_up().await;

    // And now we can use it!
    log::info!("Stack is up!");

    let mut rx_buffer = [0; 4096];
    let mut tx_buffer = [0; 4096];
    let mut buf = [0; 4096];

    loop {
        let mut socket = TcpSocket::new(stack, &mut rx_buffer, &mut tx_buffer);
        socket.set_timeout(Some(Duration::from_secs(10)));

        control.gpio_set(0, false).await;
        log::info!("Listening on TCP:1234...");
        if let Err(e) = socket.accept(1234).await {
            log::warn!("accept error: {:?}", e);
            continue;
        }

        log::info!("Received connection from {:?}", socket.remote_endpoint());
        control.gpio_set(0, true).await;

        loop {
            let n = match socket.read(&mut buf).await {
                Ok(0) => {
                    log::warn!("read EOF");
                    break;
                }
                Ok(n) => n,
                Err(e) => {
                    log::warn!("read error: {:?}", e);
                    break;
                }
            };

            log::info!("rxd {}", from_utf8(&buf[..n]).unwrap());

            match socket.write_all(&buf[..n]).await {
                Ok(()) => {}
                Err(e) => {
                    log::warn!("write error: {:?}", e);
                    break;
                }
            };
        }
    }
}