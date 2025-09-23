use {defmt_rtt as _, panic_probe as _};

use core::str::FromStr;
use cyw43::JoinOptions;
use embassy_rp::clocks::RoscRng;
use embassy_net::{
    Config,
    Stack,
    tcp::TcpSocket,
    StackResources,
};
use embassy_time::{Timer, Duration};
use embedded_io_async::Write;
use static_cell::StaticCell;
use crate::{WIFI_NETWORK, WIFI_PASSWORD, DONGLE_IP, SENDER_IP, TCP_ENDPOINT};


pub fn network_config(net_device: cyw43::NetDriver<'static>) -> (embassy_net::Stack<'static>, embassy_net::Runner<'static, cyw43::NetDriver<'static>>) {
    // Configure the network
    let config = Config::ipv4_static(embassy_net::StaticConfigV4 {
        address: embassy_net::Ipv4Cidr::new(embassy_net::Ipv4Address::from_str(SENDER_IP).unwrap(), 16),
        dns_servers: heapless::Vec::new(),
        gateway: None,
    });

    // Generate random seed
    let seed = RoscRng.next_u64();

    // Init network stack
    static RESOURCES: StaticCell<StackResources<3>> = StaticCell::new();
    let (stack, runner) = embassy_net::new(net_device, config, RESOURCES.init(StackResources::new()), seed);

    (stack, runner)
}


#[embassy_executor::task]
pub async fn tcp_client_task(mut control: cyw43::Control<'static>, stack: Stack<'static>) -> ! {
    // Try connection wifi
    while let Err(err) = control
        .join(WIFI_NETWORK, JoinOptions::new(WIFI_PASSWORD.as_bytes()))
        .await {
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

    loop {
        let mut socket = TcpSocket::new(stack, &mut rx_buffer, &mut tx_buffer);
        socket.set_timeout(Some(Duration::from_secs(10)));

        control.gpio_set(0, false).await; // LED off
        log::info!("Connecting...");
        let host_addr = embassy_net::Ipv4Address::from_str(DONGLE_IP).unwrap();
        if let Err(e) = socket.connect((host_addr, TCP_ENDPOINT)).await {
            log::warn!("connect error: {:?}", e);
            continue;
        }
        log::info!("Connected to {:?}", socket.remote_endpoint());
        control.gpio_set(0, true).await; // LED on

        let msg = b"Hello world!\n";
        loop {
            if let Err(e) = socket.write_all(msg).await {
                log::warn!("write error: {:?}", e);
                break;
            }
            log::info!("txd: {}", core::str::from_utf8(msg).unwrap());
            Timer::after_secs(1).await;
        }
    }
}