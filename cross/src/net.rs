use core::net::SocketAddr;
use cyw43::{Control, JoinOptions};
use embassy_executor::Spawner;
use embassy_net::{
    Stack,
    tcp::client::{TcpClient, TcpClientState, TcpConnection},
};
use embassy_time::{Duration, Timer, WithTimeout};
use embedded_io_async::Write as _;
use embedded_nal_async::TcpConnect;
use log::{error, info};

use crate::dht11::READING;

const SSID: &str = env!("WIFI_SSID");
const SERVER_SOCKET_ADDR: SocketAddr = SocketAddr::new(
    const_str::ip_addr!(env!("SERVER_IP")),
    const_str::parse!(env!("SERVER_PORT"), u16),
);

const WRITE_BUFFER_SIZE: usize = 128;

/// Initializes wifi and spawns net task.
pub fn spawn_net(spawner: Spawner, stack: Stack<'static>, control: &'static mut Control<'static>) {
    spawner
        .spawn(net_task(stack, control))
        .expect("unable to spawn net task");
}

#[embassy_executor::task]
async fn net_task(stack: Stack<'static>, control: &'static mut Control<'static>) {
    loop {
        let main_res = net_loop(stack, control).await;
        if let Err(e) = main_res {
            error!("net loop error: {e:?}");
            Timer::after_secs(2).await;
        }
    }
}

#[derive(Debug, thiserror::Error)]
enum NetLoopError {
    #[error("Timeout at wifi leave")]
    WifiLeaveTimeout,
    #[error("Timeout at wifi connect")]
    WifiConnectTimeout,
    #[error("Wifi connect failure status {status}")]
    WifiConnectFail { status: u32 },
    #[error("Timeout at wifi link up")]
    WifiLinkTimeout,
    #[error("Timeout at wifi config up")]
    WifiConfigTimeout,
    #[error("Unable to get IP config after max attempts")]
    IpConfigMaxAttempts,
    #[error("Timeout at TCP connect")]
    TcpConnectTimeout,
    #[error("TCP reset at TCP connect")]
    TcpConnectReset,
    #[error("Unable to write serialized message to buffer")]
    BufferWrite(#[from] postcard::Error),
    #[error("Timeout at mutex acquire")]
    MutexTimeout,
    #[error("Timeout at TCP write")]
    TcpWriteTimeout,
    #[error("TCP reset at TCP write")]
    TcpWriteConnectionReset,
}

async fn net_loop(stack: Stack<'_>, control: &mut Control<'_>) -> Result<(), NetLoopError> {
    let options = JoinOptions::new(env!("WIFI_PASSWORD").as_bytes());

    // TODO: move all timeouts to config file/env vars

    // TODO: wifi connect exponential back off

    info!("disconnecting wifi");
    control
        .leave()
        .with_timeout(Duration::from_secs(1))
        .await
        .map_err(|_e| NetLoopError::WifiLeaveTimeout)?;

    info!("connecting to wifi");
    control
        .join(SSID, options.clone())
        .with_timeout(Duration::from_secs(5))
        .await
        .map_err(|_e| NetLoopError::WifiConnectTimeout)?
        .map_err(|e| NetLoopError::WifiConnectFail { status: e.status })?;

    info!("connected to wifi {SSID}");

    info!("waiting for link to come up");
    stack
        .wait_link_up()
        .with_timeout(Duration::from_secs(5))
        .await
        .map_err(|_e| NetLoopError::WifiLinkTimeout)?;
    info!("link is up");

    info!("waiting for IP config");
    stack
        .wait_config_up()
        .with_timeout(Duration::from_secs(10))
        .await
        .map_err(|_e| NetLoopError::WifiConfigTimeout)?;

    const MAX_ATTEMPTS: usize = 5;
    let mut attempts = 0;
    'ip_config: loop {
        if attempts == MAX_ATTEMPTS {
            return Err(NetLoopError::IpConfigMaxAttempts);
        }
        if let Some(config) = stack.config_v4() {
            info!("IP config: {:?}", config);
            break 'ip_config;
        } else {
            Timer::after_millis(500).await;
        }
        attempts += 1;
    }

    let client_state = TcpClientState::<1, 1024, 1024>::new();
    let tcp_client = TcpClient::new(stack, &client_state);

    let mut socket: TcpConnection<'_, _, _, _> = tcp_client
        .connect(SERVER_SOCKET_ADDR)
        .with_timeout(Duration::from_secs(1))
        .await
        .map_err(|_e| NetLoopError::TcpConnectTimeout)?
        .map_err(|_e| NetLoopError::TcpConnectReset)?;

    // no need to periodically clear buffer -- postcard returns slice that it writes to
    // (just make sure not to read past, since it could be garbage)
    let mut buffer = [0u8; WRITE_BUFFER_SIZE];

    loop {
        // take from mutex
        let reading = {
            let mut lock = READING
                .lock()
                .with_timeout(Duration::from_secs(2))
                .await
                .map_err(|_e| NetLoopError::MutexTimeout)?;
            lock.take()
        };

        if let Some(reading) = reading {
            // postcard requires mut slice, not vec -- it doesn't call i.e. push()
            let message_bytes =
                postcard::to_slice(&reading, &mut buffer[..]).map_err(NetLoopError::BufferWrite)?;

            let bytes_written = socket
                .write(message_bytes)
                .with_timeout(Duration::from_secs(2))
                .await
                .map_err(|_e| NetLoopError::TcpWriteTimeout)?
                .map_err(|_e| NetLoopError::TcpWriteConnectionReset)?;

            info!("wrote {bytes_written} bytes to socket");
        }

        Timer::after_secs(2).await;
    }
}
