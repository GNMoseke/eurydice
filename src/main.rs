use clap::{Parser, Subcommand};
use log::info;
use rusqlite::Connection;
use std::{env, fs};

mod daemon;
mod mpd_client;
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
    env_logger::init();
    info!("MPD client initilized succesfully");
    let mut client = mpd_client::MPDClient::connect();

    let data_path = env::var("XDG_DATA_HOME")
        .unwrap_or(env::var("HOME").expect("Home env var not set") + "/.local/share/")
        + "eurydice/";
    fs::create_dir_all(&data_path)?;
    let db = Connection::open(data_path + "db.db3").expect("Could not open db connection");
    match setup_db(&db) {
        Ok(_) => {}
        Err(e) => panic!("Failed db initialization: {e:?}"),
    }

    info!("DB connection initilized succesfully");

    let args = Cli::parse();

    match args.command {
        Commands::Stats => {
            println!("stats")
        }
        Commands::Daemon => loop {
            let new_song = daemon::wait_for_song_change(&mut client);
            daemon::handle_song_change(new_song, &db);
        },
        Commands::SurpriseMe { opt } => match opt {
            SurpriseMeCommand::Album { count } => {
                let tracks = surprise_me::create_album_playlist(&db, count);
                client.add_to_queue(&tracks);
                info!(
                    "Album request - Successfully added {} tracks to playlist",
                    tracks.len()
                );
            }
            SurpriseMeCommand::Playlist {
                target_length,
                same_artist,
            } => {
                let tracks = surprise_me::create_track_playlist(&db, target_length, same_artist);
                client.add_to_queue(&tracks);
                info!(
                    "Playlist request - Successfully added {} tracks to playlist",
                    tracks.len()
                );
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
