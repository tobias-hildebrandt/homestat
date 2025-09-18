use embassy_executor::Spawner;
use embassy_net::Stack;
use embassy_time::Timer;
use log::info;

// /// Initializes wifi and spawns net task.
pub fn spawn_net(spawner: Spawner, stack: Stack<'static>) {
    spawner.spawn(net_task(stack)).unwrap();
}

#[embassy_executor::task]
async fn net_task(stack: Stack<'static>) {
    loop {
        if let Some(config) = stack.config_v4() {
            info!("{:?}", config)
        }
        Timer::after_millis(500).await;
    }
}
