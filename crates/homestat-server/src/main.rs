use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use anyhow::Context;
use homestat_wire::{WireMessage, WireMessageDisplay};
use tokio::{
    io::AsyncReadExt,
    net::{TcpListener, TcpStream},
};
use tracing::{error, info};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().init();

    // TODO: read from cmdline
    const SOCKET_ADDR: SocketAddr = SocketAddr::new(
        IpAddr::V4(Ipv4Addr::UNSPECIFIED),
        const_str::parse!(env!("SERVER_PORT"), u16),
    );

    let listener = TcpListener::bind(SOCKET_ADDR)
        .await
        .expect("unable to bind socket");

    info!("bound listener socket to address {SOCKET_ADDR}");

    let mut tasks = vec![];

    loop {
        let (stream, peer) = listener
            .accept()
            .await
            .expect("unable to accept on listener");

        let handle = tokio::spawn(peer_task(stream, peer));
        tasks.push(handle);
    }
}

async fn peer_task(mut stream: TcpStream, peer: SocketAddr) -> anyhow::Result<()> {
    info!("starting peer task for peer {peer}");

    let mut buffer = [0u8; 2048];

    loop {
        let bytes_read = stream
            .read(&mut buffer)
            .await
            .context("error reading from TCP stream")?;

        let message = &buffer[0..bytes_read];

        // try postcard
        match postcard::from_bytes::<WireMessage>(message) {
            Ok(decoded) => match decoded.inner.is_ok() {
                true => info!(
                    "{peer}: {} (0x{})",
                    WireMessageDisplay(&decoded),
                    hex::encode(message),
                ),
                false => error!(
                    "{peer}: {} (0x{})",
                    WireMessageDisplay(&decoded),
                    hex::encode(message),
                ),
            },
            Err(postcard::Error::DeserializeUnexpectedEnd) => continue,
            Err(e) => return Err(e).context("error decoding postcard"),
        }
    }
}

#[cfg(test)]
mod tests {
    use homestat_wire::{Number, Reading, WireMessage, WithTimestamp};

    #[test]
    fn print_encoded_size() {
        let reading: WireMessage = WithTimestamp {
            micros: u64::MAX,
            inner: Ok(Reading {
                temperature: Number {
                    whole: u8::MAX,
                    tenths: u8::MAX,
                },
                humidity: Number {
                    whole: u8::MAX,
                    tenths: u8::MAX,
                },
            }),
        };
        let mut buffer = [0u8; 2usize.pow(16)];
        let encoded =
            postcard::to_slice(&reading, &mut buffer[..]).expect("error encoding postcard");
        println!(
            "encoded to {} bytes: {:?} {}",
            encoded.len(),
            reading,
            hex::encode(encoded),
        );
    }
}
