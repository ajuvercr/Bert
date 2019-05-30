
extern crate futures;
extern crate tokio_core;
extern crate tokio_io;
extern crate tokio;
extern crate bert;
extern crate bytes;

use futures::prelude::*;
use futures::future::lazy;

use tokio::net::TcpStream;


use std::env;
use std::net::{SocketAddr, Shutdown};

use bert::ws::{WSReader, WSWriter};


fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 3 {
        eprintln!("WTF YO");
        eprintln!("Please specify ws address and tcp address to connect to");
        ::std::process::exit(1);
    }

    let ws_address = args[1].clone();
    let tcp_address = args[2].parse::<SocketAddr>().unwrap();

    tokio::run(lazy( move || {
        let (rx, tx) = bert::ws::connect_to(&ws_address);

        bert::Broker::new::<WSReader, WSWriter, WSReader, WSWriter>(rx, tx, 
            move || {
                // Connect to tcp address with future
                let tcp = TcpStream::connect(&tcp_address.clone());

                Box::new(tcp.map(move |stream| {
                    let stream = CloseWithShutdown(stream);
                    stream.split()
                }).map_err(|e| eprintln!("Tcp stream error {:?}", e)))
            }
        );

        Ok(())
    }));
}

use std::io::{self, Read, Write};

use tokio_io::{AsyncRead, AsyncWrite};

struct CloseWithShutdown(TcpStream);

impl Read for CloseWithShutdown {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.0.read(buf)
    }
}

impl AsyncRead for CloseWithShutdown {}

impl Write for CloseWithShutdown {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
    }
}

impl AsyncWrite for CloseWithShutdown {
    fn shutdown(&mut self) -> Poll<(), io::Error> {
        self.0.shutdown(Shutdown::Write)?;
        Ok(().into())
    }
}