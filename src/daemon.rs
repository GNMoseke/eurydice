use itertools::Itertools;
use rusqlite::Connection;
use std::collections::HashMap;
use std::env;
use std::io::{BufReader, prelude::*};
use std::os::unix::net::UnixStream;

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
        INSERT INTO tracks(title,artist,album,lengthseconds,playcount,path)
        VALUES (?1, ?2, ?3, ?4, 1, ?5)
        ON CONFLICT(title,artist,album) DO UPDATE SET playcount=playcount+1
        RETURNING id",
        )
        .unwrap();

    // queue is empty, we can just break
    if track_info.is_empty() { return }

    // FIXME: use `config` message here to pull the 'music_directory'
    let full_path = env::var("HOME").unwrap() + "/Music/" + &track_info["file"].to_string();
    song_change
        .query_one(
            [
                &track_info["Title"],
                &track_info["Artist"],
                track_info.get("Album").unwrap_or(&"Unknown Album".to_string()),
                &track_info["duration"],
                &full_path,
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

pub(crate) fn wait_for_song_change(stream: &mut UnixStream) -> String {
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
                println!("{}", current_song);
            }
            val if val.contains("ACK") => {
                println!("Unexpected MPD error: {:?}", val)
            }
            _ => panic!("Unknown update from MPD: {}", recv_str),
        }
    }

    current_song
}
