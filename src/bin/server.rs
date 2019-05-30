
extern crate futures;
extern crate tokio_core;
extern crate tokio_io;
extern crate tokio;
extern crate ws;
extern crate bert;
extern crate bytes;

use bytes::BufMut;
use ws::{Message, Sender, listen};

use futures::stream::Stream;
use futures::prelude::*;
use futures::future::lazy;
use futures::failed;
use futures::sync::mpsc;


use tokio_io::{AsyncRead, AsyncWrite};

use std::env;
use std::sync::{Arc, Mutex, MutexGuard};
use std::mem;
use std::io;

type MyF = FnMut (WSRead, WSWrite) -> ();

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("WTF BRO, pleas specify port");
        ::std::process::exit(1);
    }

    let port = args[1].clone();

    tokio::run(lazy(
        move || {
            let state = Arc::new(Mutex::new(None));
            listen(format!("0.0.0.0:{}",port), |connection| {
                println!("Got connection {:?}", connection);
                let (sender, receiver) = mpsc::channel(10);

                let reader = WSRead::new(receiver);
                let writer = WSWrite(connection.clone());

                println!("I'm here!");
                let mut state: MutexGuard<'_, Option<Box<MyF>>> = state.lock().expect("Couldn lock state");
                match *state {
                    Some(ref mut f) => f(reader, writer),
                    None => {
                        mem::replace(&mut *state, Some(Box::new(bert::Broker::<_, WSRead, WSWrite>::new(
                            reader, writer, || {
                                println!("Can't handle message, because no client");
                                Box::new(failed::<(_,_), ()>(()))
                            }
                        ))));
                    },
                }

                move |msg| {
                    let mut sender = sender.clone();
                    println!("Got ws message {:?}", msg);
                    match msg {
                        Message::Binary(bin) => sender.try_send(bin.to_vec()),
                        Message::Text(text) => sender.try_send(text.as_bytes().to_vec()),
                    }.expect("Couldn't send message");
                    Ok(())
                }
            }).unwrap();

            Ok(())
        }));

}

struct WSRead {
    inner: mpsc::Receiver<Vec<u8>>,
}

impl WSRead {
    fn new(inner: mpsc::Receiver<Vec<u8>>) -> Self {
        Self {
            inner
        }
    }
}

impl AsyncRead for WSRead {
    unsafe fn prepare_uninitialized_buffer(&self, _: &mut [u8]) -> bool {
        false
    }

    fn read_buf<B>(&mut self, buf: &mut B) -> Poll<usize, io::Error> 
        where B: BufMut {
            println!("Polling channel");
            if let Ok(Async::Ready(Some(mut buffer))) = self.inner.poll() {
                println!("Reading to buffer {:?}", buffer);
                buf.put_slice(&mut buffer);
                Ok(buffer.len().into())
            } else {
                Ok(Async::NotReady)
            }
    }
}

impl io::Read for WSRead {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        println!("Sync reading");
        if let Ok(Async::Ready(Some(mut buffer))) = self.inner.poll() {
            println!("got buffer {:?}", buffer);
            buf.swap_with_slice(&mut buffer);
            Ok(buffer.len())
        } else {
            Err(io::Error::new(io::ErrorKind::BrokenPipe, "oh no!"))
        }
    }
}

struct WSWrite(Sender);

impl AsyncWrite for WSWrite {
    fn shutdown(&mut self) -> Poll<(), io::Error> {
        Ok(Async::Ready(().into()))
    }
}

impl io::Write for WSWrite {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let vec = buf.to_vec();
        let size = vec.len();
        self.0.send(Message::Binary(vec)).map(|_| size).map_err(|e| {
            eprintln!("{:?}", e);
            io::Error::new(io::ErrorKind::BrokenPipe, "oh no!")
        })
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}