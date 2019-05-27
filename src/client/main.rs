use ws::{connect, CloseCode};

fn main() {
    let addr = format!("ws://serene-fortress-87984.herokuapp.com/");

    println!("connecting to {}", addr);

    connect(addr, |out| {
        out.send("Hello WebSocket").unwrap();

        move |msg| {
            println!("Got message: {}", msg);
            out.close(CloseCode::Normal)
        }
    }).unwrap();
}