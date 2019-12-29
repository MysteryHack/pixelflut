use tokio::prelude::*;
use tokio::net::{TcpStream};
use serde_derive::*;

use futures::prelude::*;
use std::iter::Iterator;
use futures::stream::StreamExt;

use get_if_addrs::{get_if_addrs, IfAddr};

use net2::TcpBuilder;
use std::env;

use std::sync::Arc;
use rand::prelude::*;

#[derive(Deserialize)]
struct Px {
    x: u64,
    y: u64,
    p: String
}

#[macro_export]
macro_rules! log {
    ($($rest:tt)*) => {
        println!("[{}] {}",
            ::chrono::Local::now().time().format("%H:%M:%S%.3f"),
            format_args!($($rest)*))
    };
}

// standard offset 400, 0
#[tokio::main]
async fn main() {
    // Organize command line arguments
    let args: Vec<String> = env::args().collect();
    let connections_per_ip: usize = args[1].clone().parse().unwrap();
    let x_offset = args[2].clone();
    let y_offset = args[3].clone();
    let hostname = args[4].clone();
    
    // Format payloads, remove transparent pixels
    let mut payloads: Vec<String> = read().iter()
    .filter(|px| px.p != "000000")
    .map(|px| format!("PX {} {} {}\n", px.x, px.y, px.p))
    .collect();
    
    // Retrieve IP addresses
    if let Ok(addrs) = get_if_addrs() {
        let ips: Vec<std::net::Ipv4Addr> = addrs.into_iter().filter_map(|ipaddr| {
            if !ipaddr.is_loopback() {
                if let IfAddr::V4(addr) = ipaddr.addr {
                    return Some(addr.ip);
            }    }
            None
        }).collect();

        if ips.len() > 5 {
            log!("{} IPs detected!", ips.len());

            let spacing: usize = payloads.len() / ips.len() * connections_per_ip;

            let mut rng = rand::thread_rng();
            payloads.shuffle(&mut rng);

            let tiles = payloads.chunks(spacing).map(|c| c.concat()).collect::<Vec<String>>();
            let tiles_ref = Box::leak(tiles.into_boxed_slice());
            let length = tiles_ref.len();

            futures::stream::iter(0..).for_each_concurrent(Some(connections_per_ip * ips.len()), |c| tokio::spawn(bound_work(ips[c % ips.len()], tiles_ref[c % length].clone(), x_offset.clone(), y_offset.clone(), hostname.clone())).map(drop)).await;
        }
    }

    futures::stream::iter(0..connections_per_ip).for_each_concurrent(None, |_| work(x_offset.clone(), y_offset.clone(), hostname.clone()).map(drop)).await;
}

async fn bound_work(ip: std::net::Ipv4Addr, pixels: String, x: String, y: String, hostname: String) -> Result<(),std::io::Error> {
    log!("Starting");
    let upstream = TcpBuilder::new_v4()?
        .bind((ip, 0))?
        .connect(hostname)?;

    let mut raw_socket = TcpStream::from_std(upstream)?;

    raw_socket.write_all(format!("OFFSET {} {}\n", x, y).as_ref()).await?;
    loop {
        raw_socket.write_all(pixels.as_ref()).await?;
    }
}

async fn work(x: String, y: String, hostname: String) -> Result<(),std::io::Error> {
    log!("Starting");
    let pixels = read();
    let mut raw_socket = tokio::net::TcpStream::connect(hostname).await?;
    raw_socket.write_all(format!("OFFSET {} {}\n", x, y).as_ref()).await?;
    loop {
        for pixel in &pixels {
            let payload = format!("PX {} {} {}\n", pixel.x, pixel.y, pixel.p);
            raw_socket.write_all(payload.as_ref()).await?;
        }
        log!("Cycled");
    }
}

fn read() -> Vec<Px> {
    serde_json::from_slice(&std::fs::read("img.json").unwrap()).unwrap()
}
