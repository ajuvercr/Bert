#![feature(proc_macro_hygiene, decl_macro)]
extern crate rodio;
use rodio::{Source, Sink, Decoder};

extern crate rand;
use rand::prelude::*;

#[macro_use] extern crate error_chain;
use error_chain::error_chain;

#[macro_use] extern crate rocket;
use rocket::request::Form;
use rocket::{State, };

extern crate rocket_contrib;
use rocket_contrib::json::Json;

use std::fs::File;
use std::io::BufReader;
use std::io;
use std::env;
use std::process::Command;
use std::thread;
use std::time::Duration;

use std::sync::{Arc, Mutex, };

error_chain! {
    foreign_links {
        Io(::std::io::Error) #[cfg(unix)];
    }
}

#[get("/")]
fn index() -> &'static str {
    "Hello, world!"
}


#[derive(FromForm)]
struct Request {
    url: String,
}

// The queue to play, you cannot go backwards
type Queue = Arc<Mutex<Vec<Sink>>>;

use std::path::PathBuf;
#[post("/new/<url..>")]
fn new_song(url: PathBuf, queue: State<Queue>) -> Result<()> {
    let url: String = url.iter().try_fold(String::new(), |mut acc, x| x.to_str().map(|x| {
        println!("{:?}", x);
        acc.push_str(x);
        acc.push('/');
        acc
    })).chain_err(|| "Invalid unicode")?;
    println!("url {}", url);
    let mut queue = queue.lock().map_err(|_| "Couldn't lock mutex")?;
    let sink = get_music(url).chain_err(|| "No sink acquired")?;
    queue.push(sink);

    check_music();
    Ok(())
}

fn get_music(url: String) -> Option<Sink> {
    let ext = "m4a";
    let at: u32 = rand::random();
    let filename = format!("./downloads/{}.{}", at, ext);
    let wav = format!("./downloads/{}.wav", at);

        // Download using youtube-dl
    Command::new("youtube-dl")
            .args(&["-f", "bestaudio[ext=m4a]", "-o", &filename, "--write-thumbnail", &url])
            .output()
            .expect("failed to execute process");

    println!("Converting");

    // Converting to readable wav's using ffmpeg
    Command::new("ffmpeg")
        .args(&["-i", &filename, &wav])
        .output()
        .expect("ffmpeg failed");

    println!("Playing");

    // Play with rodio
    let device = rodio::default_output_device().unwrap();

    let file = File::open(&wav).unwrap();
    let dec: Decoder<BufReader<File>> = rodio::Decoder::new(BufReader::new(file)).unwrap();

    let duration = dec.total_duration().unwrap();

    let sink = Sink::new(&device);
    sink.append(dec);
    
    None
}

fn check_music() {
    println!("'Chekcing music'");
}

fn main() -> io::Result<()> {

    let q: Queue = Arc::new(Mutex::new(Vec::new()));
    rocket::ignite()
        .manage(q)
        .mount("/", routes![index, new_song])
        .launch();


    // let ext = "m4a";

    // let mut at = 1;
    // for name in env::args().skip(1) {
    //     println!("Download {}: {}", at, name);
    //     let filename = format!("./downloads/{}.{}", at, ext);
    //     let wav = format!("./downloads/{}.wav", at);

    //     // Download using youtube-dl
    //     Command::new("youtube-dl")
    //             .args(&["-f", "bestaudio[ext=m4a]", "-o", &filename, "--write-thumbnail", &name])
    //             .output()
    //             .expect("failed to execute process");

    //     println!("Converting");

    //     // Converting to readable wav's using ffmpeg
    //     Command::new("ffmpeg")
    //         .args(&["-i", &filename, &wav])
    //         .output()
    //         .expect("ffmpeg failed");

    //     println!("Playing");

    //     // Play with rodio
    //     let device = rodio::default_output_device().unwrap();

    //     let file = File::open(&wav).unwrap();
    //     let dec: Decoder<BufReader<File>> = rodio::Decoder::new(BufReader::new(file)).unwrap();

    //     let duration = dec.total_duration().unwrap();
    //     let sink = Sink::new(&device);
    //     sink.append(dec);
        

    //     thread::sleep(duration);

    //     at += 1;
    // }

    // println!("DONE");

    // thread::sleep(Duration::from_secs(20));

    Ok(())
}
