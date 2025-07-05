use itertools::Itertools;
use rusqlite::Connection;

pub(crate) fn create_track_playlist(
    db: &Connection,
    target_length: Option<f32>,
    same_artist: bool,
) -> Vec<SelectedTrack> {
    // Default to one hour
    let target_length = target_length.unwrap_or(60.0) * 60.0;
    println!("playlist of length {:?}", target_length);

    // TODO: I'm sure there's a way to do this greedy calculation in sqlite itself
    // FIXME: I've hardcoded 300 as the limit here intentionally since it gives me a decent
    // boundary for having enough songs (works out to ~12hrs of tracks even with wide variance in
    // length). I should probably do something less dumb (see above for greedy calculation in
    // sqlite)
    // NOTE: I'm just using the average play count as a marker here (choosing things with below
    // average plays). It may be statistically more satisfying to use the median, but that's not
    // built in to sqlite and I can't imagine in a real scenario playing one song so much that it
    // skews the average.
    let query_str =
        "select * from
            (select title,artist,album,path,lengthseconds from tracks where playcount <= (select avg(playcount) from tracks) limit 300)
        order by random()"
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

pub(crate) fn create_album_playlist(db: &Connection, count: Option<u16>) -> Vec<SelectedTrack> {
    // Default to one album
    let count = count.unwrap_or(1);
    println!("playist with {:?} album(s)", count);

    // get a random set of low played albums
    let query_str = "select distinct album from tracks 
            where playcount <= (select avg(playcount) from tracks) and album != 'Unknown Album' 
            order by random() limit ?1;"
        .to_string();

    let mut query = db.prepare(query_str.as_str()).unwrap();
    let album_names: Vec<String> = query
        .query_map([&count], |row| Ok(row.get(0).unwrap()))
        .unwrap()
        .flatten()
        .collect();

    // and now query for the actual tracks
    let query_str = "select title,artist,album,path,lengthseconds from tracks where album in ("
        .to_string()
        + &album_names
            .iter()
            .map(|a| "'".to_string() + a + "'")
            .join(",")
        + ")";
    db.prepare(query_str.as_str())
        .unwrap()
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
    artist: String,
    pub(crate) album: String,
    path: String,
    length: f32,
}
