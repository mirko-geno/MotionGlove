use {defmt_rtt as _, panic_probe as _};

use core::str::FromStr;
use embassy_rp::clocks::RoscRng;
use embassy_net::{
    Config,
    Stack,
    tcp::TcpSocket,
    StackResources,
};
use embassy_sync::{
    channel::Sender,
    blocking_mutex::raw::CriticalSectionRawMutex,
};
use embedded_io_async::Write;
use static_cell::StaticCell;
use firmware::{WIFI_NETWORK, WIFI_PASSWORD, TCP_CHANNEL, TCP_ENDPOINT, SOCKET_TIMEOUT, DONGLE_IP, MessageArr, CHANNEL_SIZE};


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
pub async fn tcp_server_task(
    mut control: cyw43::Control<'static>, stack: Stack<'static>, tx_ch: Sender<'static, CriticalSectionRawMutex, MessageArr, CHANNEL_SIZE>
) -> ! {
    //control.start_ap_open("cyw43", 5).await;
    control.start_ap_wpa2(WIFI_NETWORK, WIFI_PASSWORD, TCP_CHANNEL).await;

    // And now we can use it!
    log::info!("Stack is up!");

    let mut rx_buffer = [0; 4096];
    let mut tx_buffer = [0; 4096];
    let mut buf = [0; 4096];

    loop {
        let mut socket = TcpSocket::new(stack, &mut rx_buffer, &mut tx_buffer);
        socket.set_timeout(Some(SOCKET_TIMEOUT));

        control.gpio_set(0, false).await;
        log::info!("Listening on TCP: {TCP_ENDPOINT}...");
        if let Err(e) = socket.accept(TCP_ENDPOINT).await {
            log::warn!("accept error: {:?}", e);
            continue;
        }

        log::info!("Received connection from {:?}", socket.remote_endpoint());
        control.gpio_set(0, true).await;

        loop {
            // Receives data from TCP Client
            match socket.read(&mut buf).await {
                Err(e) => {
                    log::warn!("read error: {:?}", e);
                    break;
                }
                Ok(0) => {
                    log::warn!("read EOF");
                    break;
                }
                Ok(idx) => {
                    log::info!("Received {} bytes: {:?}", idx, &buf[..idx]);
                    let mut message: MessageArr = [0;12];
                    message.copy_from_slice(&buf[..idx]); // Panics if &buf[..idx] != [u8;12]
                    tx_ch.send(message).await;
                },
            };
        }
    }
}