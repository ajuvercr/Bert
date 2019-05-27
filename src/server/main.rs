extern crate ws;

use ws::listen;

use std::env;

fn main() {
    let port = env::var("PORT").unwrap_or(String::from("80"));
    let addr = format!("wss://serene-fortress-87984.herokuapp.com/");

    println!("connecting to {}", addr);
    listen(&addr, |out| {
        move |msg| {
            println!("responding {}", msg);
            out.send(msg)
        }
    }).unwrap()
}