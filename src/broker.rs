
use futures::prelude::*;
use futures::future::lazy;
use tokio::io::{AsyncRead, AsyncWrite};
use futures::try_ready;

use std::sync::mpsc;
use std::marker::PhantomData;
use std::collections::HashMap;

type CreateClientFuture = Box<Future<Item=(Box<dyn AsyncRead>, Box<dyn AsyncWrite>), Error= ()>>;

enum Message {
    Data(u8, Vec<u8>),
    ConnectClient(Box<'static + Send + AsyncRead>, Box<'static + Send + AsyncWrite>),
}

pub struct Broker<S, F> {
    map: HashMap<u8, mpsc::Sender<Vec<u8>>>,
    handle: mpsc::Sender<Message>,
    socket: S,
    clientNotFound: F,
}

impl<S, F> Broker<S, F>
    where 
        S: 'static + Send + AsyncRead,
        F: 'static + Send + FnMut () -> CreateClientFuture {

    pub fn new(socket: S, clientNotFound: F) -> impl FnMut (Box<dyn AsyncRead>, Box<dyn AsyncWrite>) -> () {
        let (sender, receiver) = mpsc::channel();

        let broker = Broker {
            map: HashMap::new(),
            handle: sender.clone(),
            socket,
            clientNotFound
        };

        tokio::spawn(broker);

        |read, write| {
            sender.clone().send(Message::ConnectClient(read, write)).expect("Couldn't send to server");
        }
    }
}

impl<S, F> Future for Broker<S, F> 
    where S: AsyncRead,
        F: FnMut () -> CreateClientFuture {
    type Item = ();
    type Error = ();

    fn poll(&mut self) ->  Poll<Self::Item, Self::Error> {
        loop {
            let mut buffer = Vec::new();

            let size = try_ready!(
                self.socket.read_buf(&mut buffer)
                    .map_err(|e| eprintln!("{:?}", e)
                )
            );

            if size == 0  {
                return Ok(Async::Ready(()));
            }

            let id = buffer.pop().unwrap();

            self.map.get(&id).map_or_else(|| {
                    println!("No client found with id {}", id);
                    // while let Ok(_) = &(&self.clientNotFound)() {

                    // }
                }, 
                |s| s.send(buffer).expect("Coudln't send message"));

            // self.socket.send(Message(self.id, buffer)).expect("Couldn't send to server");
        }

    }
}



struct ClientReceiver<S> {
    socket: S,
    server_handle: mpsc::Sender<Message>,
    id: u8,
}

impl<S> ClientReceiver<S> {
    fn new(socket: S, server_handle: mpsc::Sender<Message>, id: u8) -> Self {
        ClientReceiver {
            socket, server_handle, id
        }
    }
}

impl<S> Future for ClientReceiver<S> 
    where S: AsyncRead {

    type Item = ();
    type Error = ();
    
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        loop {
            let mut buffer = Vec::new();

            let size = try_ready!(
                self.socket.read_buf(&mut buffer)
                    .map_err(|e| eprintln!("{:?}", e)
                )
            );

            if size == 0  {
                return Ok(Async::Ready(()));
            }

            self.server_handle.send(Message::Data(self.id, buffer)).expect("Couldn't send to server");
        }
    }
}