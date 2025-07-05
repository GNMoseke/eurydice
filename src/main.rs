use clap::{Parser, Subcommand};
use rusqlite::Connection;
use std::io::{BufReader, prelude::*};
use std::net::TcpStream;
use std::{env, fs};

mod daemon;

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

        #[arg(short, long, default_value_t = false)]
        same_artist: bool,
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
    },
}

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
        Commands::SurpriseMe {
            opt,
            same_artist,
        } => match opt {
            SurpriseMeCommand::Album { count } => {
                println!("album of count {:?}", count)
            }
            SurpriseMeCommand::Playlist { target_length } => {
                println!("playlist of length {:?}", target_length)
            }
        },
    }

    Ok(())
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
