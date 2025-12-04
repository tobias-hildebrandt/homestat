use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    time::{Duration, Instant, SystemTime},
};

use anyhow::Context;
use homestat_db::{HomestatDb, HomestatRecord, get_db_url};
use homestat_wire::{WireMessage, WireMessageDisplay};
use tokio::{
    io::AsyncReadExt,
    net::{TcpListener, TcpStream},
};
use tracing::{error, info};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().init();

    let db_url = get_db_url().expect("unable to get DB URL");
    let db = HomestatDb::new(&db_url)
        .await
        .expect("unable to connect to DB");

    info!("connected to DB at {}", db_url);

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

        let handle = tokio::spawn(peer_task(db.clone(), stream, peer));
        tasks.push(handle);
    }
}

async fn peer_task(db: HomestatDb, mut stream: TcpStream, peer: SocketAddr) -> anyhow::Result<()> {
    info!("starting peer task for peer {peer}");

    let mut buffer = [0u8; 2048];

    loop {
        let bytes_read = tokio::time::timeout(Duration::from_secs(5), stream.read(&mut buffer))
            .await
            .context("timeout waiting for tcp")?
            .context("error reading from TCP stream")?;

        let message = &buffer[0..bytes_read];

        // try postcard
        match postcard::from_bytes::<WireMessage>(message) {
            Ok(decoded) => {
                match &decoded.inner.is_ok() {
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
                }
                let record = HomestatRecord {
                    // TODO: implement source IDs
                    source_id: 1,
                    recv_timestamp: SystemTime::now().into(),
                    wire_message: decoded,
                };
                let time = Instant::now();
                tokio::time::timeout(Duration::from_secs(5), record.insert(db.as_ref()))
                    .await
                    .context("timeout waiting for db insert")?
                    .context("error inserting into db")?;
                let duration = time.elapsed();
                info!("insert to DB took {:.5} sec", duration.as_secs_f64());
            }
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
