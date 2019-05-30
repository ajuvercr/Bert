use futures::prelude::*;
use tokio::io::{AsyncRead, AsyncWrite};
use futures::try_ready;

use std::sync::mpsc;
use std::io::Cursor;

use super::Message;

pub struct ClientReceiver<S> {
    socket: S,
    server_handle: mpsc::Sender<Message>,
    id: u8,
}

impl<S> ClientReceiver<S> {
    pub fn new(socket: S, server_handle: mpsc::Sender<Message>, id: u8) -> Self {

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

pub struct ClientSender<S> {
    socket: mpsc::Receiver<Vec<u8>>,
    handle: S,
}

impl<S> ClientSender<S> {
    pub fn new(socket: mpsc::Receiver<Vec<u8>>, handle: S) -> Self {

        Self {
            socket, handle
        }
    }
}

impl<S> Future for ClientSender<S> 
    where S: AsyncWrite {

    type Item = ();
    type Error = ();
    
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        while let Ok(msg) = self.socket.try_recv() {
            self.handle.write_buf(&mut Cursor::new(msg)).expect("Coudln't send to client");
        }

        Ok(Async::NotReady)
    }
}
