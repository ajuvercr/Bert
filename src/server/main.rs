#[macro_use]
extern crate futures;
extern crate tokio_core;
extern crate tokio_io;
extern crate tokio;

use ws::{connect, CloseCode, Message, Sender, Handler, listen};

use futures::Future;
use futures::stream::Stream;
use futures::prelude::*;
use futures::future::lazy;

use tokio_io::{AsyncRead, AsyncWrite};
use tokio_io::io::{ReadHalf, WriteHalf};
use tokio_core::net::{TcpListener, TcpStream};
use tokio_core::reactor::Core;

use std::env;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::sync::mpsc;
use std::mem;

type ClientsMap = Arc<Mutex<HashMap<u8, mpsc::Sender<Msg>>>>;

#[derive(Debug)]
struct State(Option<(ClientsMap, mpsc::Sender<Msg>)>);

impl State {
    fn new() -> Self {
        Self(None)
    }
    // Add connection to self
    fn handle(&mut self, socket: Sender) -> Box<dyn Handler> {

        let (sender, reciever) = mpsc::channel();

        match &self.0 {
            Some((map, ss)) => {

                println!("Connecting to client");
                tokio::spawn(
                    ClientWriter::new(reciever, socket.clone())
                );
                
                Box::new(ClientReceiver::new(ss.clone()))
            },

            None => {

                println!("Connecting to application");
                tokio::spawn(
                        ServerWriter::new(reciever, socket.clone())
                );

                mem::replace(&mut self.0, Some((
                    Arc::new(Mutex::new(HashMap::new())),
                    sender.clone()
                )));
                
                Box::new(ServerReceiver::new(sender))

            }
        }

    }
}

#[derive(Debug)]
struct Msg {
    id: u8,
    payload: Vec<u8>
}

impl Msg {
    fn to_bytes(mut self) -> Vec<u8> {
        self.payload.push(self.id);

        self.payload
    }

    fn from_bytes(mut bytes: Vec<u8>) -> Self {
        let id = bytes.pop().unwrap();

        Msg {
            id, payload: bytes
        }
    }
}

fn main() {

    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("WTF BRO");
        eprintln!("Please specify port number");
        ::std::process::exit(1);
    }
    let port = args.get(1).map(|x| x.clone()).unwrap_or("80".to_string());
    let addr = String::from("0.0.0.0:")+&port;

    let state: Arc<Mutex<State>> = Arc::new(Mutex::new(State::new()));


    println!("connecting to {}", addr);
    listen(&addr, move |socket| {

        let state = state.clone();

        let mut s = state.lock().expect("Couldn't lock mutex");

        HandlerHandler(s.handle(socket))
    }).unwrap()
}

struct HandlerHandler(Box<dyn Handler>);

impl Handler for HandlerHandler {
    fn on_message(&mut self, msg: Message) -> ws::Result<()> {
        return self.0.on_message(msg);
    }
}

struct ServerReceiver {
    writer: mpsc::Sender<Msg>,
}

impl ServerReceiver {
    fn new(writer: mpsc::Sender<Msg>) -> Self {
        Self { writer }
    }
}

impl Handler for ServerReceiver {

    fn on_message(&mut self, msg: Message) -> ws::Result<()> {
        println!("got message {:?}", msg);
        if let Message::Binary(bin) = msg {
            self.writer.send(Msg::from_bytes(bin)).expect("Couldn't send message");
        }

        Ok(())
    }
}

struct ClientReceiver {
    writer: mpsc::Sender<Msg>,
    id: u8,
}

impl ClientReceiver {
    fn new(writer: mpsc::Sender<Msg>) -> Self {
        Self { writer, id: 0 }
    }
}

impl Handler for ClientReceiver {

    fn on_message(&mut self, msg: Message) -> ws::Result<()> {
        // TODO add identifier to message before sending to server writer
        println!("got message {:?}", msg);

        if let Message::Binary(bin) = msg {
            self.writer.send(Msg {
                id: self.id,
                payload: bin
            }).expect("Couldn't send message here");
        }

        Ok(())
    }
}

struct ServerWriter {
    reader: mpsc::Receiver<Msg>,
    sender: Sender, 
}

impl ServerWriter {
    fn new(reader: mpsc::Receiver<Msg>, sender: Sender) -> Self {
        Self {
            reader,
            sender,
        }
    }
}

impl Future for ServerWriter {
        type Item = ();
        type Error = ();

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        
        // TODO get identifier to msg before sending
        while let Ok(msg) = self.reader.try_recv() {
            println!("Sending {:?}", msg);

            match self.sender.send(Message::Text("message".to_string())) {
                Ok(()) => {},
                Err(e) => eprintln!("{:?}", e),
            }
        }

        Ok(Async::NotReady)
    }
}

struct ClientWriter {
    reader: mpsc::Receiver<Msg>,
    sender: Sender,
}

impl ClientWriter {
    fn new(reader: mpsc::Receiver<Msg>, sender: Sender) -> Self {
        Self {
            reader,
            sender,
        }
    }
}

impl Future for ClientWriter {
        type Item = ();
        type Error = ();

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {

        while let Ok(msg) = self.reader.try_recv() {
            println!("Sending {:?}", msg);

            match self.sender.send(Message::Text("message".to_string())) {
                Ok(()) => {},
                Err(e) => eprintln!("{:?}", e),
            }
        }

        Ok(Async::NotReady)
    }
}
