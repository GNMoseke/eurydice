use std::io::{BufReader, prelude::*};
use std::net::TcpStream;

fn main() -> std::io::Result<()> {
    let mut stream = TcpStream::connect("127.0.0.1:6600")?;

    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let recv: Vec<u8> = reader.fill_buf()?.to_vec();
    reader.consume(recv.len());
    let connect_ack = String::from_utf8(recv).unwrap();
    // FIXME: handle other versions
    if connect_ack != "OK MPD 0.24.0\n" {
        panic!("Unknown connection string: {}", connect_ack)
    }

    loop {
        let new_song = wait_for_song_change(&mut stream);
        handle_song_change(new_song);
    }
}

fn handle_song_change(new_song: String) {
    println!("=== NEW SONG ===\n\n {}\n=== === === === ===", new_song);
}

fn wait_for_song_change(stream: &mut TcpStream) -> String {
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    stream.write_all("currentsong\n".as_bytes()).unwrap();
    let recv = reader.fill_buf().unwrap().to_vec();
    reader.consume(recv.len());
    let mut current_song = String::from_utf8(recv).unwrap();
    let prev_song = current_song.clone();

    // TODO: don't need the entire output of currentsong if I don't want to, can just use the first
    // line ('file' key)
    // FIXME: handle errors from mpd, which are 'ACK <someinfo>'
    while prev_song == current_song {
        stream.write_all("idle player\n".as_bytes()).unwrap();
        let mut recv = reader.fill_buf().unwrap().to_vec();
        reader.consume(recv.len());
        let recv_str = String::from_utf8(recv.clone()).unwrap();

        if recv_str == "changed: player\nOK\n" {
            stream.write_all("currentsong\n".as_bytes()).unwrap();
            recv = reader.fill_buf().unwrap().to_vec();
            reader.consume(recv.len());
            current_song = String::from_utf8(recv).unwrap();
        }
    }

    current_song
}
