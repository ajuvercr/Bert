use websocket::client::ClientBuilder;
use websocket::receiver::{Reader};
use websocket::sender::Writer;
use websocket::message;

use futures::Future;
use futures::stream::Stream;
use futures::prelude::*;

use tokio_io::{AsyncRead, AsyncWrite};
use tokio_core::net::{TcpListener};
use tokio_core::reactor::Core;

use std::io::{self};
use std::env;
use std::net::TcpStream;

pub fn connect_to(ip: &str) -> (WSReader, WSWriter) {
    let client = ClientBuilder::new(ip)
        .unwrap()
        .add_protocol("rust-websocket")
        .connect_insecure()
        .unwrap();

    let (tx, rx) = client.split().unwrap();

    (WSReader(tx), WSWriter(rx))
}

pub struct WSReader(Reader<TcpStream>);

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

pub struct WSWriter(Writer<TcpStream>);

impl AsyncWrite for WSWriter {

    fn shutdown(&mut self) -> Poll<(), io::Error> {
        Ok(Async::Ready(()))
    }
}

impl io::Write for WSWriter {

    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let buf = buf.to_vec();

        let length = buf.len();
        self.0.send_message(&message::OwnedMessage::Binary(buf))
            .map(|_| length)
            .map_err(|e| {eprintln!("error {:?}", e); io::Error::new(io::ErrorKind::BrokenPipe, "woopsy")})
    }

    fn flush(&mut self) -> io::Result<()> {
        self.0.stream.flush()
    }
}
