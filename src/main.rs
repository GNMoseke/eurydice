use clap::{Parser, Subcommand};
use log::{error, info};
use rusqlite::Connection;
use std::{env, fs};

use crate::collection::CollectionFormat;

mod collection;
mod daemon;
mod mpd_client;
mod never_played;
mod stats;
mod surprise_me;

#[derive(Debug, Parser)]
#[command(
    version,
    about = "Rediscover your muisc.",
    long_about = "
eurydice is a small sidecar client to MPD that simply records track listen history and can be used
to generate a random playlist of less frequently played music, either as a \"mixtape\" of tracks or
as entire albums.
"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    #[command(arg_required_else_help = true)]
    #[command(about = "Queue up a set of less-played tracks or albums")]
    SurpriseMe {
        #[command(subcommand)]
        opt: SurpriseMeCommand,
    },
    #[command(about = "Output some interesting stats about played tracks")]
    Stats,
    #[command(
        about = "List out tracks that are in the collection, but have no plays in the history."
    )]
    NeverPlayed,
    #[command(about = "Start the eurydice daemon to record MPD play history.")]
    Daemon,
    #[command(about = "collection information")]
    Collection {
        #[arg(short, long, help = "Output Format")]
        format: Option<CollectionFormat>,
    },
}

#[derive(Debug, Subcommand)]
enum SurpriseMeCommand {
    #[command(about = "Add one or more less-played albums to your queue")]
    Album {
        #[arg(
            short,
            long,
            help = "Number of albums to queue up. If not given, one album will be added to the queue."
        )]
        count: Option<u16>,
    },
    #[command(about = "Add a \"mixtape\" of less-played songs to your queue")]
    Playlist {
        #[arg(short, long, help = "Length of playlist to build, in minutes")]
        target_length: Option<f32>,

        #[arg(
            short,
            long,
            default_value_t = false,
            help = "Enforce that all tracks must come from the same artist (which will be selected randomly)"
        )]
        same_artist: bool,
    },
}

fn main() -> std::io::Result<()> {
    env_logger::init();
    info!("MPD client initilized succesfully");
    let mut client = mpd_client::MPDClient::connect();
    let config_response = client
        .send_command("config\n".to_string())
        .expect("MPD config message returned unexpected response");
    let mpd_music_dir = config_response
        .lines()
        .next()
        .expect("MPD config message returned no data")
        .split(": ")
        .last()
        .expect("MPD config message did not contain music directory");
    let music_dir = std::path::Path::new(&mpd_music_dir);

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
            // TODO: pass params through for limit and unique etc
            stats::print_stats_table(&db);
        }
        Commands::NeverPlayed => match never_played::never_played(&db, &mut client, music_dir) {
            Ok(tracks) => println!("{tracks}"),
            Err(e) => {
                println!("Could not find unplayed tracks");
                error!("Error finding unplayed tracks: {e}")
            }
        },
        Commands::Collection { format } => match format {
            Some(format) => println!(
                "{}",
                collection::collection_information(&mut client, music_dir, format)
            ),
            None => println!(
                "{}",
                collection::collection_information(
                    &mut client,
                    music_dir,
                    CollectionFormat::Summary
                )
            ),
        },
        Commands::Daemon => loop {
            let new_song = daemon::wait_for_song_change(&mut client);
            // This is *technically* recoverable (though the daemon will likely be in an unideal
            // state). In the future could kill daemon after 10 song change failures in a row or
            // something.
            daemon::handle_song_change(new_song, &db, music_dir)
                .unwrap_or_else(|err| error!("Error during song change handle: {err:?}"))
        },
        Commands::SurpriseMe { opt } => match opt {
            SurpriseMeCommand::Album { count } => {
                let tracks = surprise_me::create_album_playlist(&db, count)
                    .unwrap_or_else(|err| panic!("Error creating mixtape: {err:?}"));
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
                let tracks = surprise_me::create_track_playlist(&db, target_length, same_artist)
                    .unwrap_or_else(|err| panic!("Error creating mixtape: {err:?}"));
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
