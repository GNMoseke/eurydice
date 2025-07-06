use crate::mpd_client::MPDClient;
use itertools::Itertools;
use rusqlite::Connection;
use std::collections::HashMap;
use std::env;

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

    // queue is empty, we can just break
    if track_info.is_empty() {
        return;
    }

    let mut song_change = db
        .prepare(
            "
        INSERT INTO tracks(title,artist,album,lengthseconds,playcount,path)
        VALUES (?1, ?2, ?3, ?4, 1, ?5)
        ON CONFLICT(title,artist,album) DO UPDATE SET playcount=playcount+1
        RETURNING id",
        )
        .unwrap();

    // FIXME: use `config` message here to pull the 'music_directory'
    let full_path = env::var("HOME").unwrap() + "/Music/" + &track_info["file"].to_string();
    song_change
        .query_one(
            [
                &track_info["Title"],
                &track_info["Artist"],
                track_info
                    .get("Album")
                    .unwrap_or(&"Unknown Album".to_string()),
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

pub(crate) fn wait_for_song_change(client: &mut MPDClient) -> String {
    let mut current_song = match client.send_command("currentsong\n".to_string()) {
        Some(song) => song,
        None => "".to_string(),
    };
    let prev_song = current_song.clone();

    // TODO: don't need the entire output of currentsong if I don't want to, can just use the first
    // line ('file' key)
    while prev_song == current_song {
        if let Some(val) = client.send_command("idle player\n".to_string())
            && val == "changed: player\n"
        {
            current_song = match client.send_command("currentsong\n".to_string()) {
                Some(song) => song,
                None => "".to_string(),
            };
        }
    }

    current_song
}
