extern crate tokio;

use tokio::io;
use tokio::net::TcpListener;
use tokio::prelude::*;

use std::env;

fn main() {
    let port = env::var("PORT").unwrap_or(String::from("80"));
    let addr = format!("127.0.0.1:{}", port).parse().unwrap();
    let listener = TcpListener::bind(&addr).unwrap();

    // Here we convert the `TcpListener` to a stream of incoming connections
    // with the `incoming` method. We then define how to process each element in
    // the stream with the `for_each` combinator method
    let server = listener.incoming().for_each(|socket| {
    // split the socket stream into readable and writable parts
        let (reader, writer) = socket.split();
        // copy bytes from the reader into the writer
        let amount = io::copy(reader, writer);

        let msg = amount.then(|result| {
            match result {
                Ok((amount, _, _)) => println!("wrote {} bytes", amount),
                Err(e)             => println!("error: {}", e),
            }

            Ok(())
        });

        // spawn the task that handles the client connection socket on to the
        // tokio runtime. This means each client connection will be handled
        // concurrently
        tokio::spawn(msg);
        Ok(())
    })
    .map_err(|err| {
        // Handle error by printing to STDOUT.
        println!("accept error = {:?}", err);
    });

    println!("server running on localhost:{}", port);

    // Start the server
    //
    // This does a few things:
    //
    // * Start the Tokio runtime
    // * Spawns the `server` task onto the runtime.
    // * Blocks the current thread until the runtime becomes idle, i.e. all
    //   spawned tasks have completed.
    tokio::run(server);
}