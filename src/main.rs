use itertools::Itertools;
use rusqlite::Connection;
use std::collections::HashMap;
use std::io::{BufReader, prelude::*};
use std::net::TcpStream;
use std::{env, fs, os, path};

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

    let data_path = env::var("XDG_DATA_HOME")
        .unwrap_or(env::var("HOME").unwrap() + "/.local/share/")
        + "eurydice/";
    fs::create_dir_all(&data_path)?;
    let db = Connection::open(data_path + "db.db3").unwrap();

    db.execute(
        "CREATE TABLE IF NOT EXISTS tracks (
            id INTEGER PRIMARY KEY,
            title TEXT,
            artist TEXT,
            album TEXT,
            lengthseconds REAL,
            playcount INTEGER,
            UNIQUE (title, artist, album)
        )",
        (),
    )
    .unwrap();

    db.execute(
        "CREATE TABLE IF NOT EXISTS history (
            timeunix INTEGER,
            songid INTEGER,
            FOREIGN KEY (songid) REFERENCES tracks(id)
        )",
        (),
    )
    .unwrap();

    loop {
        let new_song = wait_for_song_change(&mut stream);
        handle_song_change(new_song, &db);
    }
    //
    // Ok(())
}

fn handle_song_change(new_song: String, db: &Connection) {
    println!("=== NEW SONG ===\n\n {}\n=== === === === ===", new_song);
    let track_info: HashMap<String, String> = new_song
        .trim()
        .split('\n')
        .dropping_back(1) // Drop the "OK"
        .map(|l| {
            l.split_once(':')
                .map(|(a, b)| (a.trim().to_string(), b.trim().to_string()))
                .unwrap()
        })
        .collect();
    let mut song_change = db
        .prepare(
            "
        INSERT INTO tracks(title,artist,album,lengthseconds,playcount)
        VALUES (?1, ?2, ?3, ?4, 1)
        ON CONFLICT(title,artist,album) DO UPDATE SET playcount=playcount+1
        RETURNING id, playcount",
        )
        .unwrap();
    song_change
        .query_one(
            [
                &track_info["Title"],
                &track_info["Artist"],
                &track_info["Album"],
                &track_info["duration"],
            ],
            |id| {
                let retid: i32 = id.get(0).unwrap();
                let count: i32 = id.get(1).unwrap();
                println!("{:?}", retid);
                println!("{:?}", count);
                Ok(())
            },
        )
        .unwrap();
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
