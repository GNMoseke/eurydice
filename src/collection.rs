use clap::ValueEnum;
use glob::glob;
use log::warn;
use serde::Serialize;
use serde_json::Result;
use std::{collections::HashMap, env, path::Path};

use crate::mpd_client::MPDClient;

#[derive(Debug, Clone)]
// for clap
#[derive(Copy, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub(crate) enum CollectionFormat {
    Summary,
    Rofi,
    Json,
    Detailed,
}

#[derive(Debug, Serialize)]
enum IndexedItemType {
    Album,
    Track,
    Playlist,
}

#[derive(Debug, Serialize)]
struct IndexedItem {
    path: String,
    cover_path: Option<String>,
    item_type: IndexedItemType,
    title: String,
    artist: String,
}

// FIXME: unwraps
pub(crate) fn collection_information(client: &mut MPDClient, format: CollectionFormat) -> String {
    // 1. find all flac, add to dict
    // 2. find everything else, add to dict
    // theoretically that keeps flac where possible and falls back to the other formats if
    // necessary
    let all_flac = client
        .send_command("find \"(file contains \'.flac\')\" sort AlbumSort\n".to_string())
        .unwrap();
    let remainder = client
        .send_command("find \"(!(file contains \'.flac\'))\" sort AlbumSort\n".to_string())
        .unwrap();

    let (flac_tracks, flac_albums) = parse_info(all_flac.trim().split("file:").collect());
    let (mut tracks, mut albums) = parse_info(remainder.trim().split("file:").collect());

    // TODO: also list playlists
    // Should allow image thumbnail to be provided by config file

    // NOTE: since extend overwrites existing keys with new values, we extend the lower quality
    // dict to upsert to the better quality tracks
    tracks.extend(flac_tracks);
    albums.extend(flac_albums);

    match format {
        CollectionFormat::Summary => todo!(),
        CollectionFormat::Rofi => {
            tracks.extend(albums);
            let rofi_strs: Vec<String> = tracks
                .iter()
                .map(|item| match &item.1.cover_path {
                    Some(cover) => item.0.to_owned() + "\\u0000icon\\u001fthumbnail://" + cover,
                    None => item.0.to_owned(),
                })
                .collect();
            rofi_strs.join("\n")
        }
        CollectionFormat::Json => {
            tracks.extend(albums);
            serde_json::to_string(&tracks).unwrap()
        }
        CollectionFormat::Detailed => todo!(),
    }
}

fn parse_info(
    all_track_details: Vec<&str>,
) -> (HashMap<String, IndexedItem>, HashMap<String, IndexedItem>) {
    // "TRACK: Artist - Title": "rel/path/to/file"
    let mut tracks = HashMap::<String, IndexedItem>::new();
    // "Album: Artist - Title": "rel/path/to/album/dir"
    let mut albums = HashMap::<String, IndexedItem>::new();

    all_track_details.iter().for_each(|ti| {
        // FIXME: this little thing to prepend the key back on (which I need for the dictionary
        // below) can definitely be done better, there's a way to maintain it during the split
        // operation I'm sure. Can't use split_inclusive.
        let corrected = "file:".to_string() + ti;
        let track_info: HashMap<String, String> = corrected
            .trim()
            .split('\n')
            .map(|l| {
                l.split_once(':')
                    .map(|(a, b)| (a.trim().to_string(), b.trim().to_string()))
                    .unwrap_or_else(|| {
                        warn!("Failed to parse track info from {ti}");
                        ("".to_string(), "".to_string())
                    })
            })
            .collect();

        let unknown = "Unknown".to_string();

        // NOTE: have to handle both Artist and AlbumArtist tags here. Some tracks have one or
        // the other, some have both. Take the first one we find.
        let mut track_key = "TRACK: ".to_string();
        let mut album_key = "ALBUM: ".to_string();
        let artist = match (track_info.get("AlbumArtist"), track_info.get("Artist")) {
            (Some(a), _) => a,
            (_, Some(a)) => a,
            (None, None) => &unknown
        };

        track_key += artist;
        album_key += artist;
        track_key += " - ";
        album_key += " - ";

        // TODO: this does some weird stuff with singles where they are indexed as both a track
        // (ok) and an "Unknown" album. Not hugely annoying right now but to be aware of.
        let track_title = track_info.get("Title").unwrap_or(&unknown);
        let album_title = track_info.get("Album").unwrap_or(&unknown);
        track_key += track_title;
        album_key += album_title;

        let file_path = track_info.get("file").unwrap_or_else(|| {
            warn!("No file path found for {track_key}");
            &unknown
        });

        // NOTE: probably best to avoid using the MPD protocol `albumart` message since it returns
        // raw bytes and I want the path to the actual image.
        // FIXME: unwraps
        let mut cover_path: Option<String> = None;
        if let Some(album_dir_path) = Path::new(file_path).parent() {
            let dir_path_string = album_dir_path.to_str().unwrap().to_string();
            // FIXME: use `config` message here to pull the 'music_directory' or pull from a eurydice
            // config file
            let cover_path_glob = Path::new(&(env::var("HOME").unwrap() + "/Music/")).join(album_dir_path.join("**/cover.*"));
            for entry in glob(cover_path_glob.to_str().unwrap()).unwrap() {
                match entry {
                    Ok(path) => cover_path = Some(path.to_str().unwrap().to_string()),
                    Err(_) => continue
                }
            }
            albums.insert(album_key, IndexedItem { path: dir_path_string, cover_path: cover_path.clone(), item_type: IndexedItemType::Album, artist: artist.to_string(), title: album_title.to_string() });
        } else {
            warn!("Could not find album directory for {file_path}, album and cover art will not be returned");
        }
        tracks.insert(track_key, IndexedItem { path: file_path.to_string(), cover_path, item_type: IndexedItemType::Track, artist: artist.to_string(), title: track_title.to_string() });
    });

    (tracks, albums)
}
