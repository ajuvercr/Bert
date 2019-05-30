
extern crate futures;
extern crate tokio_core;
extern crate tokio_io;
extern crate tokio;
extern crate bert;
extern crate websocket;

use websocket::client::ClientBuilder;
use websocket::receiver::{Reader};
use websocket::sender::Writer;

use futures::Future;
use futures::stream::Stream;
use futures::prelude::*;

use tokio_io::{AsyncRead, AsyncWrite};
use tokio_core::net::{TcpListener};
use tokio_core::reactor::Core;

use std::io::{self};
use std::env;
use std::net::TcpStream;

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("WTF BRO");
        eprintln!("Please specify inner and outer address");
        ::std::process::exit(1);
    }
    let inner_address = args[1].parse().unwrap();
    let outer_address = args[2].clone();

    let mut core = Core::new().unwrap();
    let handle = core.handle();

    let socket = TcpListener::bind(&inner_address, &handle).unwrap();
    println!("Listening on: {:?}", inner_address);

    let done = socket.incoming()
        .map(move |(socket, addr)| {
            println!("Got connection from {:?}", addr);
            let client = ClientBuilder::new(&outer_address.clone())
                .unwrap()
                .add_protocol("rust-websocket")
                .connect_insecure()
                .unwrap();
            (socket, client)
        })
        .for_each(move |(tcp_socket, ws_socket)| {
            println!("Extra connection made");

            let (tcp_tx, tcp_rx) = tcp_socket.split();
            let (ws_tx, ws_rx) = ws_socket.split().unwrap();

            tokio::spawn(tokio::io::copy(tcp_tx, WSWriter(ws_rx)).map(|(size, _, _)| println!("copied {} bytes", size)).map_err(|e| eprintln!("oops {:?}", e)));
            tokio::spawn(tokio::io::copy(WSReader(ws_tx), tcp_rx).map(|(size, _, _)| println!("copied {} bytes", size)).map_err(|e| eprintln!("oops {:?}", e)));

            Ok(())
        });

    core.run(done).unwrap();

    Ok(())
}

struct WSReader(Reader<TcpStream>);

impl AsyncRead for WSReader {

    unsafe fn prepare_uninitialized_buffer(&self, _: &mut [u8]) -> bool {
        false
    }
}

impl io::Read for WSReader {

    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {

        if let Ok(size) = self.0.stream.get_ref().peek(buf) {
            if size == 0 {
                return Ok(0)
            }
        }

        self.0.recv_message().map(|msg| {
            match msg {
                message::OwnedMessage::Binary(bin) => {
                    buf[..bin.len()].copy_from_slice(&bin);
                    bin.len()
                },
                _ => {
                    eprintln!("euhm erroring? Got message {:?}", msg);
                    0
                }
            }
        })
        .map_err(|e| {eprintln!("error {:?}", e); io::Error::new(io::ErrorKind::BrokenPipe, "woopsy")})
    }
}

struct WSWriter<R: io::Write>(Writer<R>);

impl<R> AsyncWrite for WSWriter<R>
    where R: io::Write {

    fn shutdown(&mut self) -> Poll<(), io::Error> {
        Ok(Async::Ready(()))
    }
}

use websocket::message;
impl<R> io::Write for WSWriter<R>
    where R: io::Write  {

    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let buf = buf.to_vec();
        println!("Writing {:?}", buf);
        let length = buf.len();
        self.0.send_message(&message::OwnedMessage::Binary(buf))
            .map(|_| length)
            .map_err(|e| {eprintln!("error {:?}", e); io::Error::new(io::ErrorKind::BrokenPipe, "woopsy")})
    }

    fn flush(&mut self) -> io::Result<()> {
        self.0.stream.flush()
    }
}
