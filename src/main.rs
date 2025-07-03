use rusqlite::Connection;
use std::io::{BufReader, prelude::*};
use std::net::TcpStream;
use std::{env, fs};

mod daemon;

fn main() -> std::io::Result<()> {
    let mut stream = TcpStream::connect(
        env::var("MPD_HOST").unwrap_or("localhost".to_string())
            + ":"
            + &env::var("MPD_PORT").unwrap_or("6600".to_string()),
    )?;

    let mut reader = BufReader::new(stream.try_clone().expect("MPD connection invalid"));
    let recv: Vec<u8> = reader.fill_buf()?.to_vec();
    reader.consume(recv.len());
    let connect_ack = String::from_utf8(recv).expect("MPD connection invalid");

    // NOTE: Protocol version agnostic here, see:
    // https://mpd.readthedocs.io/en/latest/protocol.html#protocol-overview
    if !connect_ack.contains("OK MPD") {
        panic!("Unknown connection string: {}", connect_ack)
    }

    let data_path = env::var("XDG_DATA_HOME")
        .unwrap_or(env::var("HOME").expect("No Data path found") + "/.local/share/")
        + "eurydice/";
    fs::create_dir_all(&data_path)?;
    let db = Connection::open(data_path + "db.db3").expect("Could not open db connection");
    match setup_db(&db) {
        Ok(_) => {}
        Err(e) => panic!("Failed db initialization: {:?}", e),
    }

    loop {
        let new_song = daemon::wait_for_song_change(&mut stream);
        daemon::handle_song_change(new_song, &db);
    }
    //
    // Ok(())
}

fn setup_db(db: &Connection) -> std::result::Result<(), rusqlite::Error> {
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
    )?;

    db.execute(
        "CREATE TABLE IF NOT EXISTS history (
            time DATETIME DEFAULT CURRENT_TIMESTAMP,
            songid INTEGER,
            FOREIGN KEY (songid) REFERENCES tracks(id)
        )",
        (),
    )?;

    Ok(())
}
