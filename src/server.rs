use futures::prelude::*;
use tokio::io::{AsyncRead};
use futures::try_ready;

use std::sync::mpsc;

use super::Message;

pub struct ServerReceiver<S> {
    socket: S,
    server_handle: mpsc::Sender<Message>,
}

impl<S> ServerReceiver<S> {
    pub fn new(socket: S, server_handle: mpsc::Sender<Message>) -> Self {

        Self {
            socket, server_handle
        }
    }
}

impl<S> Future for ServerReceiver<S> 
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

            let id = buffer.pop().unwrap();

            self.server_handle.send(Message::ToClient(id, buffer)).expect("Couldn't send to server");
        }
    }
}