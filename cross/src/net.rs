use cyw43::{Control, JoinOptions};
use embassy_executor::Spawner;
use embassy_net::Stack;
use embassy_time::Timer;
use log::info;

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

    loop {
        if let Some(config) = stack.config_v4() {
            info!("{:?}", config)
        }
        Timer::after_millis(5000).await;
    }
}
