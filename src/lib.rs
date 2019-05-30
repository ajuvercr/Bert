
use futures::prelude::*;
use tokio::io::{AsyncRead, AsyncWrite};
use atomic_counter::RelaxedCounter;
use atomic_counter::AtomicCounter;

use std::sync::mpsc;
use std::marker::PhantomData;
use std::collections::HashMap;

mod client;
mod server;

type CreateClientFuture<R, W> = Box<Future<Item=(R, W), Error= ()> + Send>;

pub enum Message {
    ToClient(u8, Vec<u8>),
    ToServer(u8, Vec<u8>),
    ConnectClient(u8, mpsc::Sender<Vec<u8>>),
}

pub struct Broker<F, R, W> {
    map: HashMap<u8, mpsc::Sender<Vec<u8>>>,
    server_write: mpsc::Sender<Vec<u8>>,
    handle: mpsc::Sender<Message>,
    receiver: mpsc::Receiver<Message>,
    client_not_found: F,
    phantom_r: PhantomData<R>,
    phantom_w: PhantomData<W>,
}

impl<F, R, W> Broker<F, R, W>
    where 
        R: 'static + Send + AsyncRead,
        W: 'static + Send + AsyncWrite,
        F: 'static + Send + FnMut () -> CreateClientFuture<R, W> {

    pub fn new<SR, SW, RR, WW> (socket_reader: SR, socket_writer: SW, client_not_found: F) -> impl FnMut (RR, WW) -> ()
        where 
            SR: 'static + Send + AsyncRead,
            SW: 'static + Send + AsyncWrite,
            RR: 'static + Send + AsyncRead,
            WW: 'static + Send + AsyncWrite {

        let (sender, receiver) = mpsc::channel();

        let broker = Broker {
            map: HashMap::new(),
            server_write: client::ClientSender::new(socket_writer),
            handle: sender.clone(),
            receiver,
            client_not_found,
            phantom_r: PhantomData,
            phantom_w: PhantomData,
        };

        tokio::spawn(server::ServerReceiver::new(socket_reader, sender.clone()));

        tokio::spawn(broker);

        let counter = RelaxedCounter::new(0);

        move |read, write| {
            counter.inc();
            create_client(read, write, counter.get() as u8, sender.clone());
        }
    }

}

fn create_client<R, W> (read: R, write: W, id: u8, handle: mpsc::Sender<Message>)
    where 
        R: 'static + Send + AsyncRead,
        W: 'static + Send + AsyncWrite {
    tokio::spawn(client::ClientReceiver::new(read, handle.clone(), id));

    let sender = client::ClientSender::new(write);

    handle.send(Message::ConnectClient(id, sender)).expect("Couldn't send message to server");
}

impl<F, R, W> Future for Broker<F, R, W> 
    where
        R: 'static + Send + AsyncRead,
        W: 'static + Send + AsyncWrite,
        F: FnMut () -> CreateClientFuture<R, W> + Send  {
    type Item = ();
    type Error = ();

    fn poll(&mut self) ->  Poll<Self::Item, Self::Error> {
        loop {

            while let Ok(msg) = self.receiver.try_recv() {
                match msg {
                    Message::ToClient(id, msg) => {
                        match self.map.get(&id) {
                            Some(channel) => {
                                channel.send(msg).expect("Couldn't send to client channel");
                            },
                            None => {
                                tokio::spawn({
                                    let handle = self.handle.clone();

                                    (self.client_not_found)().map(move |(read, write)| {
                                        create_client(read, write, id, handle.clone());
                                        ()
                                    })
                                });
                            }
                        }
                    },
                    Message::ToServer(id, mut msg) => {
                        msg.push(id);
                        self.server_write.send(msg).expect("Couldn't write to server");
                    },
                    Message::ConnectClient(id, channel) => {
                        if self.map.contains_key(&id) {
                            eprintln!("Already found client with id {}", id);
                        } else {
                            self.map.insert(id, channel);
                        }
                    },
                }
            }
        }

    }
}

