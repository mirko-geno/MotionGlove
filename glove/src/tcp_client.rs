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
use embassy_sync::{
    channel::Receiver,
    blocking_mutex::raw::CriticalSectionRawMutex,
};
use embassy_time::{Timer, Duration, with_timeout};
use embedded_io_async::Write;
use static_cell::StaticCell;

use shared::{
    definitions::{
        WIFI_NETWORK, WIFI_PASSWORD,
        DONGLE_IP, GLOVE_IP,
        TCP_ENDPOINT,
        SOCKET_TIMEOUT,
        CHANNEL_SIZE,
    },
    custom_hid::HidInstruction
};


pub fn network_config(net_device: cyw43::NetDriver<'static>) -> (embassy_net::Stack<'static>, embassy_net::Runner<'static, cyw43::NetDriver<'static>>) {
    // Configure the network
    let config = Config::ipv4_static(embassy_net::StaticConfigV4 {
        address: embassy_net::Ipv4Cidr::new(embassy_net::Ipv4Address::from_str(GLOVE_IP).unwrap(), 16),
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
pub async fn tcp_client_task(
mut control: cyw43::Control<'static>, stack: Stack<'static>, rx_ch: Receiver<'static, CriticalSectionRawMutex, HidInstruction, CHANNEL_SIZE>
) -> ! {
    let mut rx_buffer = [0; 4096];
    let mut tx_buffer = [0; 4096];
    let mut hid_instruction: HidInstruction;

    // Try wifi connection
    loop {
        log::info!("Connecting to WiFi...");
        control.leave().await; // Drops any wifi association to avoid control.join(...) crashes
        // with_timeout to retry avoiding softlocks
        match with_timeout(Duration::from_secs(5), 
        control.join(WIFI_NETWORK, JoinOptions::new(WIFI_PASSWORD.as_bytes()))).await {
            Err(_err) => {
                log::info!("Wifi connection failed, connection timed out");
                Timer::after_millis(250).await;
                continue;
            },

            Ok(res) => {
                if let Err(err) = res {
                    log::info!("Wifi connection failed with status={}", err.status);
                    Timer::after_millis(250).await;
                    continue;
                }
            }
        }

        log::info!("Waiting for link...");
        stack.wait_link_up().await;

        log::info!("Waiting for DHCP...");
        stack.wait_config_up().await;

        // Ready to use!
        log::info!("Stack is up!");

        // Clean buffers
        rx_buffer.fill(0);
        tx_buffer.fill(0);

        loop {
            let mut socket = TcpSocket::new(stack, &mut rx_buffer, &mut tx_buffer);
            socket.set_timeout(Some(SOCKET_TIMEOUT));

            control.gpio_set(0, false).await; // LED off
            log::info!("Connecting to TCP...");
            let host_addr = embassy_net::Ipv4Address::from_str(DONGLE_IP).unwrap();
            if let Err(e) = socket.connect((host_addr, TCP_ENDPOINT)).await {
                log::warn!("TCP connection error: {:?}", e);
                break;
            }

            log::info!("Connected to {:?}", socket.remote_endpoint());
            control.gpio_set(0, true).await; // LED on

            // Communication loop
            loop {
                hid_instruction = rx_ch.receive().await;
                let tcp_message = hid_instruction.to_be_bytes();
                if let Err(e) = socket.write_all(&tcp_message).await {
                    log::warn!("Write error: {:?}", e);
                    break;
                }
                log::info!("sent: {:?}", hid_instruction);
            }
        }
    }
}