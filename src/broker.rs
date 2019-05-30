
use futures::prelude::*;
use futures::future::lazy;
use tokio::io::{AsyncRead, AsyncWrite};
use futures::try_ready;

use std::sync::mpsc;
use std::marker::PhantomData;
use std::collections::HashMap;

struct Message(u8, Vec<u8>);

pub struct Broker<S> {
    map: HashMap<u8, mpsc::Sender<Vec<u8>>>,
    handle: mpsc::Sender<Message>,
    socket: S,
    clientNotFound: Box<Fn()->Result<(), ()>>,
}

impl<S> Broker<S> {

    pub fn new(socket: S, clientNotFound: Box<Fn()->Result<(), ()>>) -> Self {
        let (sender, receiver) = mpsc::channel();

        Broker {
            map: HashMap::new(),
            handle: sender,
            socket,
            clientNotFound
        }
    }

    pub fn add_connection<SS>(&mut self, socket: SS) -> Option<mpsc::Receiver<Vec<u8>>> 
        where SS: 'static + AsyncRead + Send {
        let id = 0; // TODO: generate id
        let (sender, receiver) = mpsc::channel();

        self.map.insert(id, sender);

        tokio::spawn(
            IncomingHandler::new(socket, self.handle.clone(), id)
        );

        Some(receiver)
    }

}

impl<S> Future for Broker<S> 
    where S: AsyncRead {
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
                    while let Ok(_) = (&self.clientNotFound)() {

                    }
                }, 
                |s| s.send(buffer).expect("Coudln't send message"));

            // self.socket.send(Message(self.id, buffer)).expect("Couldn't send to server");
        }

    }
}


struct IncomingHandler<S> {
    socket: S,
    server_handle: mpsc::Sender<Message>,
    id: u8,
}

impl<S> IncomingHandler<S> {
    fn new(socket: S, server_handle: mpsc::Sender<Message>, id: u8) -> Self {
        IncomingHandler {
            socket, server_handle, id
        }
    }
}

impl<S> Future for IncomingHandler<S> 
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

            self.server_handle.send(Message(self.id, buffer)).expect("Couldn't send to server");
        }
    }
}