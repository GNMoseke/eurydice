use itertools::Itertools;
use rusqlite::{Connection, Statement};

pub(crate) fn create_track_playlist(
    db: &Connection,
    target_length: Option<f32>,
    same_artist: bool,
) -> Vec<SelectedTrack> {
    // Default to one hour
    let target_length = target_length.unwrap_or(60.0) * 60.0;
    println!("playlist of length {:?}", target_length);

    // TODO: I'm sure there's a way to do this greedy calculation in sqlite itself
    let query_str =
        "select * from
            (select title,artist,album,path,lengthseconds from tracks where playcount <= (select avg(playcount) from tracks) limit 300)
        order by random();"
            .to_string();

    let mut query = db.prepare(query_str.as_str()).unwrap();
    let mut random_tracks: Vec<SelectedTrack> = query
        .query_map([], |row| {
            Ok(SelectedTrack {
                title: row.get(0)?,
                artist: row.get(1)?,
                album: row.get(2)?,
                path: row.get(3)?,
                length: row.get(4)?,
            })
        })
        .unwrap()
        .flatten()
        .collect();

    // just take the first artist for simplicity
    if same_artist {
        let artist = &random_tracks.first().unwrap().artist;
        random_tracks = random_tracks
            .iter()
            .filter(|t| t.artist == *artist)
            .map(|t| t.to_owned())
            .collect()
    }

    // greedily take until we reach the target length
    let mut sum = 0.0;
    random_tracks
        .iter()
        .take_while(|t| {
            sum += t.length;
            sum <= target_length
        })
        .map(|t| t.to_owned())
        .collect()
}

pub(crate) fn create_album_playlist(
    db: &Connection,
    count: Option<u16>,
    same_artist: bool,
) -> Vec<SelectedTrack> {
    // Default to one album
    let count = count.unwrap_or(1);
    println!("playist with {:?} album(s)", count);

    let mut query_str = "
        SELECT t1.title,t1.artist,t1.album,t1.path FROM tracks t1 INNER JOIN tracks t2 on t2.id=t1.id
        GROUP BY t1.id HAVING sum(t2.lengthseconds) <= 3600 ORDER BY t1.playcount".to_string();

    if same_artist {
        query_str += ",t1.artist";
    }

    let mut query = db.prepare(query_str.as_str()).unwrap();
    query
        .query_map([], |row| {
            Ok(SelectedTrack {
                title: row.get(0)?,
                artist: row.get(1)?,
                album: row.get(2)?,
                path: row.get(3)?,
                length: row.get(4)?,
            })
        })
        .unwrap()
        .flatten()
        .collect()
}

#[derive(Debug, Clone)]
pub(crate) struct SelectedTrack {
    pub(crate) title: String,
    pub(crate) artist: String,
    pub(crate) album: String,
    pub(crate) path: String,
    pub(crate) length: f32,
}
