
#[macro_use]
extern crate futures;
extern crate tokio_core;
extern crate tokio_io;
extern crate tokio;

use ws::{connect, CloseCode, Message, Sender, Handler};

use futures::Future;
use futures::stream::Stream;
use futures::prelude::*;

use tokio_io::{AsyncRead, AsyncWrite};
use tokio_io::io::{ReadHalf, WriteHalf};
use tokio_core::net::{TcpListener, TcpStream};
use tokio_core::reactor::Core;

use std::io::{self, Write};
use std::env;
use std::sync::{Arc, Mutex};
use std::mem;

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        println!("WTF BRO");
        println!("Please specify inner and outer address");
        ::std::process::exit(1);
    }
    let inner_address = args[1].parse().unwrap();
    let outer_address = args[2].clone();

    let mut core = Core::new().unwrap();
    let handle = core.handle();

    let socket = TcpListener::bind(&inner_address, &handle).unwrap();
    println!("Listening on: {:?}", inner_address);

    let done = socket.incoming().for_each(move |(socket, addr)| {
        println!("got conntection from {}", addr);
        let socket = Arc::new(Mutex::new(Some(socket)));
        let outer_address = outer_address.clone();

        connect(outer_address.clone(), move |out| {
            println!("connecting to {}", outer_address.clone());

            let socket = mem::replace(
                &mut *socket.clone().lock().map_err(|_| ()).unwrap(), 
                None
            ).unwrap();

            let (reader, writer) = socket.split();

            println!("here");

            tokio::spawn(
                WriteFuture::new(reader, out.clone())
            );

            Writer { writer }
        }).map_err(|e| println!("failed {:?}",e )).unwrap();

        Ok(())
    });


    core.run(done).unwrap();

    Ok(())
}

struct Writer<S> {
    writer: WriteHalf<S>
}

impl<S> Handler for Writer<S> 
    where S: AsyncWrite {

    fn on_message(&mut self, msg: Message) -> ws::Result<()> {
        println!("got message {:?}", msg);

        match msg {
            Message::Binary(bin) => self.writer.write_all(&bin),
            Message::Text(msg) => self.writer.write_all(msg.as_bytes()),
        }.expect("Couldn't write to tcp stream");

        Ok(())
    }
}

struct WriteFuture<S> {
    reader: ReadHalf<S>,
    sender: Sender,
}

impl<S> WriteFuture<S> {
    fn new(reader: ReadHalf<S>, sender: Sender) -> Self 
        where S: std::fmt::Debug {
        WriteFuture {
            reader,
            sender,
        }
    }
}

impl<S> Future for WriteFuture<S>
    where S: AsyncRead {
        type Item = ();
        type Error = ();

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        println!("polling");

        loop {
            let mut buffer = Vec::new();

            let size = try_ready!(self.reader.read_buf(&mut buffer).map_err(|e| println!("{:?}", e)));

            let msg = String::from_utf8(buffer.clone());

            println!("got {:?} with size {}", msg, size);

            self.sender.send(Message::Binary(buffer))
                .map_err(|e| println!("{:?}", e)).unwrap();
        }
    }
}

