use std::collections::HashSet;
use std::path::Path;

use itertools::Itertools;
use rusqlite::{Connection, fallible_iterator::FallibleIterator};

use crate::{collection::build_collection_maps, mpd_client::MPDClient};

pub(crate) fn never_played(
    db: &Connection,
    client: &mut MPDClient,
    music_dir: &Path,
) -> Result<String, rusqlite::Error> {
    let query = "select title || ' - ' || artist from tracks".to_string();
    let played_tracks: HashSet<String> = db
        .prepare(&query)?
        .query([])?
        .map(|r| r.get(0))
        .collect()
        .unwrap();
    let (all_tracks, _) = build_collection_maps(client, music_dir);
    // PERF: this clone is unnecessary but this command is a one off
    let all_tracks_set: HashSet<String> = all_tracks
        .iter()
        .map(|t| t.1.title.clone() + " - " + &t.1.artist)
        .collect();

    let unplayed = all_tracks_set.difference(&played_tracks).join("\n");

    Ok(unplayed)
}
