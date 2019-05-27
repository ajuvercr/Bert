extern crate ws;

use ws::listen;

use std::env;

fn main() {
    let port = env::var("PORT").unwrap_or(String::from("80"));
    let addr = format!("0.0.0.0:{}", port);

    listen(&addr, |out| {
        move |msg| {
        out.send(msg)
    }
    }).unwrap()
}