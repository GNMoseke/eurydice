use itertools::Itertools;
use rusqlite::Connection;
use std::collections::HashMap;
use std::io::{BufReader, prelude::*};
use std::net::TcpStream;

pub(crate) fn handle_song_change(new_song: String, db: &Connection) {
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
        RETURNING id",
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
                db.execute("INSERT INTO history(songid) VALUES (?1)", [&retid])
                    .unwrap();
                Ok(())
            },
        )
        .unwrap();
}

pub(crate) fn wait_for_song_change(stream: &mut TcpStream) -> String {
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    stream.write_all("currentsong\n".as_bytes()).unwrap();
    let recv = reader.fill_buf().unwrap().to_vec();
    reader.consume(recv.len());
    let mut current_song = String::from_utf8(recv).unwrap();
    let prev_song = current_song.clone();

    // TODO: don't need the entire output of currentsong if I don't want to, can just use the first
    // line ('file' key)
    while prev_song == current_song {
        stream.write_all("idle player\n".as_bytes()).unwrap();
        let mut recv = reader.fill_buf().unwrap().to_vec();
        reader.consume(recv.len());
        let recv_str = String::from_utf8(recv.clone()).unwrap();

        match recv_str {
            val if val == *"changed: player\nOK\n" => {
                stream.write_all("currentsong\n".as_bytes()).unwrap();
                recv = reader.fill_buf().unwrap().to_vec();
                reader.consume(recv.len());
                current_song = String::from_utf8(recv).unwrap();
            }
            val if val.contains("ACK") => {
                println!("Unexpected MPD error: {:?}", val)
            }
            _ => panic!("Unknown update from MPD: {}", recv_str),
        }
    }

    current_song
}
