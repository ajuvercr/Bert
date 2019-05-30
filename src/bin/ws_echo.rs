extern crate ws;

use ws::{connect};

use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("WTF BRO, pleas specify port");
        ::std::process::exit(1);
    }

    let ip = args[1].clone();

    connect(ip, |out| {
        println!("Connected to {:?}", out);
        move |msg| {
            println!("Got message {:?}", msg);
            out.send(msg).expect("Couldn't send msg");
            Ok(())
        }
    }).unwrap();
}