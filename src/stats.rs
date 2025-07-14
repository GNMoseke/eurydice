use colored::Colorize;
use itertools::Itertools;
use log::{debug, trace};
use rusqlite::Connection;
use tabled::{builder::Builder, settings::Style};

fn total_playtime(db: &Connection) -> String {
    let query =
        "select sum(lengthseconds) from tracks inner join history on tracks.id = history.songid"
            .to_string();
    let dur = db
        .prepare(&query)
        .unwrap()
        .query_map([], |row| {
            Ok(std::time::Duration::from_secs_f64(row.get(0)?))
        })
        .unwrap()
        .flatten()
        .last()
        .unwrap();
    let sec = dur.as_secs() % 60;
    let min = (dur.as_secs() / 60) % 60;
    let hr = (dur.as_secs() / 60) / 60;
    format!("{hr:0>2}:{min:0>2}:{sec:0>2}").to_string().bold().green().to_string()
}

fn most_played_track(db: &Connection, limit: u32, since: Option<String>) -> String {
    let query = match since {
        None => "select artist,album,title,playcount from tracks order by playcount desc limit ?1"
            .to_string(),
        Some(boundary) => todo!(),
    };
    db.prepare(&query)
        .unwrap()
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
        })
        .unwrap()
        .flatten()
        .join("\n")
}

fn most_played_albums(db: &Connection, limit: u32) -> String {
    // NOTE: only considered a "full" listen if every song in the album was listened to - so we
    // take the minimum here of the playcount
    let query = format!(
        "select album,min(playcount) as p from tracks group by album order by p desc limit {limit}"
    )
    .to_string();
    "todo".to_string()
}

fn most_played_artists(db: &Connection, limit: u32) -> String {
    // FIXME: incorrect query
    //let query = format!("select artist,sum(playcount) as p from tracks group by artist order by p desc limit {limit}").to_string();
    "todo".to_string()
}

fn track_count(db: &Connection, unique: bool) -> String {
    let query = match unique {
        true => "select count(*) from tracks",
        false => "select count(*) from history",
    }
    .to_string();
    let c: usize = db.prepare(&query)
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .flatten()
        .last()
        .unwrap();
    c.to_string().bold().green().to_string()
}

fn album_count(db: &Connection, unique: bool) -> String {
    "todo".to_string()
}

fn artist_count(db: &Connection) -> String {
    "todo".to_string()
}

pub(crate) fn print_stats_table(db: &Connection) {
    let mut table_builder = Builder::with_capacity(7, 2);
    [
        vec!["Total Playtime".italic().to_string(), total_playtime(db)],
        vec!["Total Track Listens".italic().to_string(), track_count(db, false)],
        vec!["Unique Track Listens".italic().to_string(), track_count(db, true)],
        vec!["Total Album Count".italic().to_string(), album_count(db, false)],
        vec!["Total Artist Count".italic().to_string(), artist_count(db)],
        vec![
            "Most Played Tracks".italic().to_string(),
            most_played_track(db, 5, None),
        ],
        vec!["Most Played Albums".italic().to_string(), most_played_albums(db, 5)],
        vec![
            "Most Played Artists".italic().to_string(),
            most_played_artists(db, 5),
        ],
    ]
    .iter()
    .for_each(|r| table_builder.push_record(r));
    let mut table = table_builder.build();
    let printable = table.with(Style::modern_rounded());
    println!("{printable}")
}
