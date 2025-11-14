use core::{fmt::Write as _, net::SocketAddr};
use cyw43::{Control, JoinOptions};
use embassy_executor::Spawner;
use embassy_net::{
    Stack,
    tcp::client::{TcpClient, TcpClientState, TcpConnection},
};
use embassy_time::{Duration, Timer, WithTimeout};
use embedded_io_async::Write as _;
use embedded_nal_async::TcpConnect;
use heapless::Vec;
use log::{error, info};

use crate::dht11::READING;

const SSID: &str = env!("WIFI_SSID");

/// Initializes wifi and spawns net task.
pub fn spawn_net(spawner: Spawner, stack: Stack<'static>, control: &'static mut Control<'static>) {
    // TODO: move to build-time parse -- in build.rs or a proc macro
    let server_address = env!("SERVER_ADDRESS")
        .parse()
        .expect("unable to parse server address");

    spawner
        .spawn(net_task(stack, control, server_address))
        .expect("unable to spawn net task");
}

#[embassy_executor::task]
async fn net_task(
    stack: Stack<'static>,
    control: &'static mut Control<'static>,
    server_address: SocketAddr,
) {
    loop {
        let main_res = net_loop(stack, control, server_address).await;
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
    BufferWrite,
    #[error("Timeout at mutex acquire")]
    MutexTimeout,
    #[error("Timeout at TCP write")]
    TcpWriteTimeout,
    #[error("TCP reset at TCP write")]
    TcpWriteConnectionReset,
}

async fn net_loop(
    stack: Stack<'_>,
    control: &mut Control<'_>,
    server_address: SocketAddr,
) -> Result<(), NetLoopError> {
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
        .connect(server_address)
        .with_timeout(Duration::from_secs(1))
        .await
        .map_err(|_e| NetLoopError::TcpConnectTimeout)?
        .map_err(|_e| NetLoopError::TcpConnectReset)?;

    let mut buffer = MessageBuffer::<256>::default();

    loop {
        buffer.0.clear();
        {
            let mut lock = READING
                .lock()
                .with_timeout(Duration::from_secs(2))
                .await
                .map_err(|_e| NetLoopError::MutexTimeout)?;
            let reading = lock.take();
            // TODO: postcard
            writeln!(buffer, "{:?}", reading).map_err(|_e| NetLoopError::BufferWrite)?;
        }

        let bytes_written = socket
            .write(&buffer.0)
            .with_timeout(Duration::from_secs(2))
            .await
            .map_err(|_e| NetLoopError::TcpWriteTimeout)?
            .map_err(|_e| NetLoopError::TcpWriteConnectionReset)?;

        info!("wrote {bytes_written} bytes to socket");

        Timer::after_secs(2).await;
    }
}

#[derive(Default)]
struct MessageBuffer<const SIZE: usize>(Vec<u8, SIZE>);

impl<const SIZE: usize> core::fmt::Write for MessageBuffer<SIZE> {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.0
            .extend_from_slice(s.as_bytes())
            .map_err(|_e| core::fmt::Error)
    }
}
