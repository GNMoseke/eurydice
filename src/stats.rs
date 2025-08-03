use colored::Colorize;
use itertools::Itertools;
use log::error;
use rusqlite::Connection;
use tabled::{builder::Builder, settings::Style};

fn total_playtime(db: &Connection) -> Result<std::time::Duration, rusqlite::Error> {
    let query =
        "select sum(lengthseconds) from tracks inner join history on tracks.id = history.songid"
            .to_string();
    Ok(db
        .prepare(&query)?
        .query_map([], |row| {
            Ok(std::time::Duration::from_secs_f64(row.get(0)?))
        })?
        .flatten()
        .last()
        .unwrap_or(std::time::Duration::new(0, 0)))
}

fn most_played_track(
    db: &Connection,
    limit: u32,
    since: Option<String>,
) -> Result<String, rusqlite::Error> {
    let query = match since {
        None => "select artist,album,title,playcount from tracks order by playcount desc limit ?1"
            .to_string(),
        Some(_boundary) => todo!(),
    };
    Ok(db
        .prepare(&query)?
        .query_map([limit], |row| {
            Ok(format!(
                "{} - {} - {}: {}",
                row.get(0)
                    .unwrap_or("Unknown Artist".to_string())
                    .italic()
                    .red(),
                row.get(1)
                    .unwrap_or("Unknown Album".to_string())
                    .italic()
                    .blue(),
                row.get(2)
                    .unwrap_or("No Title".to_string())
                    .italic()
                    .purple(),
                row.get(3).unwrap_or(0).to_string().bold().green()
            ))
        })?
        .flatten()
        .join("\n"))
}

fn most_played_albums(db: &Connection, limit: u32) -> Result<String, rusqlite::Error> {
    let query = "select artist,album,sum(playcount) as p from tracks group by album order by p desc limit ?1".to_string();
    Ok(db
        .prepare(&query)?
        .query_map([limit], |row| {
            Ok(format!(
                "{} - {}: {}",
                row.get(0)
                    .unwrap_or("Unknown Artist".to_string())
                    .italic()
                    .red(),
                row.get(1)
                    .unwrap_or("Unknown Album".to_string())
                    .italic()
                    .blue(),
                row.get(2).unwrap_or(0).to_string().bold().green()
            ))
        })?
        .flatten()
        .join("\n"))
}

fn most_played_artists(db: &Connection, limit: u32) -> Result<String, rusqlite::Error> {
    let query =
        "select artist,sum(playcount) as p from tracks group by artist order by p desc limit ?1"
            .to_string();
    Ok(db
        .prepare(&query)?
        .query_map([limit], |row| {
            Ok(format!(
                "{}: {}",
                row.get(0)
                    .unwrap_or("Unknown Artist".to_string())
                    .italic()
                    .red(),
                row.get(1).unwrap_or(0).to_string().bold().green()
            ))
        })?
        .flatten()
        .join("\n"))
}

fn track_count(db: &Connection, unique: bool) -> Result<i32, rusqlite::Error> {
    let query = match unique {
        true => "select count(*) from tracks",
        false => "select count(*) from history",
    }
    .to_string();
    Ok(db
        .prepare(&query)?
        .query_map([], |row| row.get(0))?
        .flatten()
        .last()
        .unwrap_or(-1))
}

fn album_count(db: &Connection) -> Result<i32, rusqlite::Error> {
    let query = "select count(distinct album) from tracks".to_string();
    Ok(db
        .prepare(&query)?
        .query_map([], |row| row.get(0))?
        .flatten()
        .last()
        .unwrap_or(-1))
}

fn artist_count(db: &Connection) -> Result<i32, rusqlite::Error> {
    let query = "select count(distinct artist) from tracks".to_string();
    Ok(db
        .prepare(&query)?
        .query_map([], |row| row.get(0))?
        .flatten()
        .last()
        .unwrap_or(-1))
}

pub(crate) fn print_stats_table(db: &Connection) {
    let mut table_builder = Builder::with_capacity(7, 2);
    [
        vec![
            "Total Playtime".italic().to_string(),
            match total_playtime(db) {
                Ok(time) => {
                    let sec = time.as_secs() % 60;
                    let min = (time.as_secs() / 60) % 60;
                    let hr = (time.as_secs() / 60) / 60;

                    format!("{hr:0>2}:{min:0>2}:{sec:0>2}")
                }
                Err(err) => {
                    error!("Failed to calculate total playtime: {err:?}");
                    "Unknown".to_string()
                }
            }
            .bold()
            .green()
            .to_string(),
        ],
        vec![
            "Total Track Listens".italic().to_string(),
            match track_count(db, false) {
                Ok(-1) => "Unknown".to_string(),
                Ok(ct) => ct.to_string(),
                Err(err) => {
                    error!("Failed to calculate track listens: {err:?}");
                    "Unknown".to_string()
                }
            }
            .bold()
            .green()
            .to_string(),
        ],
        vec![
            "Unique Track Listens".italic().to_string(),
            match track_count(db, true) {
                Ok(-1) => "Unknown".to_string(),
                Ok(ct) => ct.to_string(),
                Err(err) => {
                    error!("Failed to calculate unique track listens: {err:?}");
                    "Unknown".to_string()
                }
            }
            .bold()
            .green()
            .to_string(),
        ],
        vec![
            "Total Albums In Eurydice Collection".italic().to_string(),
            match album_count(db) {
                Ok(-1) => "Unknown".to_string(),
                Ok(ct) => ct.to_string(),
                Err(err) => {
                    error!("Failed to calculate total album count: {err:?}");
                    "Unknown".to_string()
                }
            }
            .bold()
            .green()
            .to_string(),
        ],
        vec![
            "Total Artists In Eurydice Collection".italic().to_string(),
            match artist_count(db) {
                Ok(-1) => "Unknown".to_string(),
                Ok(ct) => ct.to_string(),
                Err(err) => {
                    error!("Failed to calculate total artist count: {err:?}");
                    "Unknown".to_string()
                }
            }
            .bold()
            .green()
            .to_string(),
        ],
        vec![
            "Most Played Tracks".italic().to_string(),
            match most_played_track(db, 5, None) {
                Ok(tracks) => tracks,
                Err(err) => {
                    error!("Failed to calculate most played tracks: {err:?}");
                    "Unknown".to_string()
                }
            },
        ],
        vec![
            "Most Played Albums\n(By Total Track Listens)"
                .italic()
                .to_string(),
            match most_played_albums(db, 5) {
                Ok(tracks) => tracks,
                Err(err) => {
                    error!("Failed to calculate most played albums: {err:?}");
                    "Unknown".to_string()
                }
            },
        ],
        vec![
            "Most Played Artists".italic().to_string(),
            match most_played_artists(db, 5) {
                Ok(tracks) => tracks,
                Err(err) => {
                    error!("Failed to calculate most played artists: {err:?}");
                    "Unknown".to_string()
                }
            },
        ],
    ]
    .iter()
    .for_each(|r| table_builder.push_record(r));
    let mut table = table_builder.build();
    let printable = table.with(Style::modern_rounded());
    println!("{printable}")
}
