use clap::{Parser, Subcommand};
use itertools::Itertools;
use rusqlite::Connection;
use std::io::{BufReader, prelude::*};
use std::os::unix::net::UnixStream;
use std::{env, fs};

mod daemon;
mod surprise_me;

#[derive(Debug, Parser)]
#[command(version, about="Rediscover your muisc.", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    #[command(arg_required_else_help = true)]
    SurpriseMe {
        #[command(subcommand)]
        opt: SurpriseMeCommand,
    },
    Stats,
    Daemon,
}

#[derive(Debug, Subcommand)]
enum SurpriseMeCommand {
    Album {
        #[arg(short, long, help = "Number of albums to queue up")]
        count: Option<u16>,
    },
    Playlist {
        #[arg(short, long, help = "Length of playlist to build, in minutes")]
        target_length: Option<f32>,

        #[arg(short, long, default_value_t = false)]
        same_artist: bool,
    },
}

fn main() -> std::io::Result<()> {
    // NOTE: MUST use a unix socket to manage the queue locally. This is "documented" in the mpd
    // protocal manual here: https://mpd.readthedocs.io/en/latest/client.html#introduction
    // where "local socket" means "unix socket".
    // See also: https://github.com/MusicPlayerDaemon/MPD/issues/2184
    let mut stream = UnixStream::connect(
        env::var("XDG_RUNTIME_DIR").unwrap_or("/run".to_string()) + "/mpd/socket",
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
        .unwrap_or(env::var("HOME").expect("Home env var not set") + "/.local/share/")
        + "eurydice/";
    fs::create_dir_all(&data_path)?;
    let db = Connection::open(data_path + "db.db3").expect("Could not open db connection");
    match setup_db(&db) {
        Ok(_) => {}
        Err(e) => panic!("Failed db initialization: {:?}", e),
    }

    let args = Cli::parse();

    match args.command {
        Commands::Stats => {
            println!("stats")
        }
        Commands::Daemon => loop {
            let new_song = daemon::wait_for_song_change(&mut stream);
            daemon::handle_song_change(new_song, &db);
        },
        Commands::SurpriseMe { opt } => match opt {
            SurpriseMeCommand::Album { count } => {
                let tracks = surprise_me::create_album_playlist(&db, count);
                tracks
                    .iter()
                    .for_each(|t| println!("{:?} - {:?}", t.title, t.album));
                add_to_queue(tracks, &mut stream);
            }
            SurpriseMeCommand::Playlist {
                target_length,
                same_artist,
            } => {
                let tracks = surprise_me::create_track_playlist(&db, target_length, same_artist);
                tracks
                    .iter()
                    .for_each(|t| println!("{:?} - {:?}", t.title, t.album));
                add_to_queue(tracks, &mut stream);
            }
        },
    }

    Ok(())
}

fn add_to_queue(tracks: Vec<surprise_me::SelectedTrack>, stream: &mut UnixStream) {
    stream.write_all("command_list_begin\n".as_bytes()).unwrap();
    stream
        .write_all(
            (tracks
                .iter()
                .map(|t| "add \"".to_string() + &t.path + "\"")
                .join("\n")
                + "\n")
                .as_bytes(),
        )
        .unwrap();
    stream.write_all("command_list_end\n".as_bytes()).unwrap();
    stream.flush().unwrap();

    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let recv = reader.fill_buf().unwrap().to_vec();
    reader.consume(recv.len());
    println!("{}", String::from_utf8(recv).unwrap())
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
            path TEXT,
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
