extern crate rodio;

use std::fs::File;
use std::io::BufReader;
use rodio::Source;

use std::io;
use std::env;
use std::process::Command;
use std::thread;
use std::time::Duration;

fn main() -> io::Result<()> {
    let ext = "m4a";

    let mut at = 1;
    for name in env::args().skip(1) {
        println!("Download {}: {}", at, name);
        let filename = format!("./downloads/{}.{}", at, ext);
        let wav = format!("./downloads/{}.wav", at);

        // Download using youtube-dl
        Command::new("youtube-dl")
                .args(&["-f", "bestaudio[ext=m4a]", "-o", &filename, "--write-thumbnail", &name])
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
        let dec: rodio::Decoder<BufReader<File>> = rodio::Decoder::new(BufReader::new(file)).unwrap();

        let duration = dec.total_duration().unwrap();
        let sink = rodio::Sink::new(&device);
        sink.append(dec);
        

        thread::sleep(duration);

        at += 1;
    }

    println!("DONE");

    thread::sleep(Duration::from_secs(20));

    Ok(())
}
