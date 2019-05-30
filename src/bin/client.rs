
extern crate futures;
extern crate tokio_core;
extern crate tokio_io;
extern crate tokio;
extern crate bert;
extern crate websocket;

use websocket::client::ClientBuilder;

use futures::Future;
use futures::stream::Stream;

use tokio_io::{AsyncRead};
use tokio_core::net::{TcpListener};
use tokio_core::reactor::Core;

use std::io::{self};
use std::env;

use bert::ws;

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
        .for_each(move |(socket, addr)| {
            
            println!("Got connection from {:?}", addr);
            let (ws_tx, ws_rx) = ws::connect_to(&outer_address);

            let (tcp_tx, tcp_rx) = socket.split();

            tokio::spawn(tokio::io::copy(tcp_tx, ws_rx).map(|(size, _, _)| println!("copied {} bytes", size)).map_err(|e| eprintln!("oops {:?}", e)));
            tokio::spawn(tokio::io::copy(ws_tx, tcp_rx).map(|(size, _, _)| println!("copied {} bytes", size)).map_err(|e| eprintln!("oops {:?}", e)));

            Ok(())
        });

    core.run(done).unwrap();

    Ok(())
}