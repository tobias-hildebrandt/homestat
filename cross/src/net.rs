use cyw43::{Control, JoinOptions};
use embassy_executor::Spawner;
use embassy_net::{
    Stack,
    tcp::client::{TcpClient, TcpClientState, TcpConnection},
};
use embassy_time::Timer;
use embedded_io_async::Write;
use embedded_nal_async::TcpConnect;
use log::{error, info};

// /// Initializes wifi and spawns net task.
pub fn spawn_net(spawner: Spawner, stack: Stack<'static>, control: &'static mut Control<'static>) {
    spawner.spawn(net_task(stack, control)).unwrap();
}

#[embassy_executor::task]
async fn net_task(stack: Stack<'static>, control: &'static mut Control<'static>) {
    let options = JoinOptions::new(env!("WIFI_PASSWORD").as_bytes());
    let res = control.join(env!("WIFI_SSID"), options).await;

    info!("wifi connect?: {:?}", res);

    info!("waiting for link...");
    stack.wait_link_up().await;

    info!("waiting for DHCP...");
    stack.wait_config_up().await;

    // loop {
    if let Some(config) = stack.config_v4() {
        info!("{:?}", config)
    }
    Timer::after_millis(5000).await;
    // }

    let client_state = TcpClientState::<1, 1024, 1024>::new();
    let tcp_client = TcpClient::new(stack, &client_state);

    let mut socket: Option<TcpConnection<'_, _, _, _>> = None;

    let server_address = env!("SERVER_ADDRESS").parse().unwrap();

    loop {
        if socket.is_none() {
            let res = tcp_client.connect(server_address).await;
            match res {
                Ok(s) => socket = Some(s),
                Err(e) => {
                    error!("TCP connect failed: {e:?}");
                    Timer::after_secs(2).await;
                    continue;
                }
            }
        }
        let socket = socket.as_mut().unwrap();

        match socket.write(b"test 123 test 123\n").await {
            Ok(bytes_written) => info!("wrote {bytes_written}bytes to socket"),
            Err(e) => error!("TCP socket write failed: {e:?}"),
        }

        Timer::after_secs(2).await;
    }
}
