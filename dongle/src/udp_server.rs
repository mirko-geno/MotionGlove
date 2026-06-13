use {defmt_rtt as _, panic_probe as _};

use core::str::FromStr;
use embassy_rp::clocks::RoscRng;
use embassy_net::{
    Config,
    Stack,
    udp::{UdpSocket, PacketMetadata},
    StackResources,
};
use embassy_sync::{
    channel::Sender,
    blocking_mutex::raw::CriticalSectionRawMutex,
};
use cyw43::JoinOptions;
use embassy_time::Timer;
use static_cell::StaticCell;

use shared::{
    definitions::{
        WIFI_NETWORK, WIFI_PASSWORD,
        DONGLE_IP,
        // TCP_CHANNEL,
        CONNECTION_ENDPOINT,
        SOCKET_TIMEOUT,
        CHANNEL_SIZE
    },
    custom_hid::HidInstruction
};


pub fn network_config(net_device: cyw43::NetDriver<'static>) -> (embassy_net::Stack<'static>, embassy_net::Runner<'static, cyw43::NetDriver<'static>>) {
    // Configure the network
    let config = Config::ipv4_static(embassy_net::StaticConfigV4 {
        address: embassy_net::Ipv4Cidr::new(embassy_net::Ipv4Address::from_str(DONGLE_IP).unwrap(), 16),
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
pub async fn udp_server_task(
    mut control: cyw43::Control<'static>, stack: Stack<'static>, tx_ch: Sender<'static, CriticalSectionRawMutex, HidInstruction, CHANNEL_SIZE>
) -> ! {
    /* Create access point instead of connecting to WIFI in this way:
    //control.start_ap_open("cyw43", 5).await;
    control.start_ap_wpa2(WIFI_NETWORK, WIFI_PASSWORD, TCP_CHANNEL).await;

    // And now we can use it!
    log::info!("Stack is up!");

    let mut rx_buffer = [0; 4096];
    let mut tx_buffer = [0; 4096];
    let mut buf = [0; 4096];
    */

    // Try network connection
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
    let mut rx_meta = [PacketMetadata::EMPTY; 16];
    let mut tx_meta = [PacketMetadata::EMPTY; 16];
    let mut buf = [0; 4096];

    
    loop {
        control.gpio_set(0, false).await; // LED off
        log::info!("Listening on UDP: {CONNECTION_ENDPOINT}...");
        
        let mut socket = UdpSocket::new(
            stack, &mut rx_meta, &mut rx_buffer, &mut tx_meta, &mut tx_buffer
        );
    
        while let Err(e) = socket.bind(CONNECTION_ENDPOINT) {
            log::warn!("Couldn't bind endpoint, error: {:?}", e);
            Timer::after_millis(10).await;
        };
        
        if !socket.may_recv() {
            log::info!("Waiting for connection");
            // Wait for connection
            while !socket.may_recv() {
                Timer::after_millis(10).await;
            }
        }

        log::info!("Received connection on endpoint {:?}", socket.endpoint());
        control.gpio_set(0, true).await; // LED on

        loop {
            // Executes when data is received from UDP Client, Err() wraps timeout error,
            // Ok() wraps recv Result
            match embassy_time::with_timeout(
                SOCKET_TIMEOUT,
                socket.recv_from(&mut buf)
            ).await {
                Err(_) => {
                    log::warn!("UDP Connection timeout");
                    break
                }
                Ok(Err(e)) => {
                    log::warn!("read error: {:?}", e);
                    break
                }
                Ok(Ok((idx, _))) => {
                    let received = &buf[..idx];
                    // log::info!("Received {} bytes: {:?}", idx, received);
                    let (chunks, _) = received.as_chunks::<16>(); // As chunks of len 16
                    for chunk in chunks {
                        let hid_instruction = HidInstruction::from_be_bytes(*chunk);
                        tx_ch.send(hid_instruction).await;
                    };
                },
            };
        }
    }
}