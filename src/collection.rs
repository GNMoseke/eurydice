use clap::ValueEnum;
use colored::Colorize;
use itertools::Itertools;
use log::warn;
use serde::Serialize;
use std::{collections::HashMap, env, path::Path};
use tabled::{Table, builder::Builder};

use crate::mpd_client::MPDClient;

#[derive(Debug, Clone)]
// for clap
#[derive(Copy, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub(crate) enum CollectionFormat {
    Summary,
    Rofi,
    Json,
    Fixmes,
}

#[derive(Debug, Serialize, PartialEq)]
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

    // NOTE: since extend overwrites existing keys with new values, we extend the lower quality
    // dict to upsert to the better quality tracks
    tracks.extend(flac_tracks);
    albums.extend(flac_albums);

    match format {
        CollectionFormat::Summary => build_summary_table(tracks, albums).to_string(),
        CollectionFormat::Rofi => {
            tracks.extend(albums);
            add_custom_items(client, &mut tracks);

            let rofi_strs: Vec<String> = tracks
                .iter()
                .map(|item| match &item.1.cover_path {
                    Some(cover) => item.0.to_owned() + "\0icon\x1fthumbnail://" + cover,
                    None => item.0.to_owned(),
                })
                .collect();
            rofi_strs.join("\n")
        }
        CollectionFormat::Json => {
            tracks.extend(albums);
            add_custom_items(client, &mut tracks);
            serde_json::to_string(&tracks).unwrap()
        }
        CollectionFormat::Fixmes => {
            // TODO: albums without flac?
            let albums_without_cover = albums
                .iter()
                .filter(|(_, v)| v.cover_path.is_none())
                .map(|(k, _)| format!("\t{k}"))
                .join("\n");
            "Alubms Without Cover Art:\n"
                .bold()
                .underline()
                .red()
                .to_string()
                + &albums_without_cover
        }
    }
}

fn add_custom_items(client: &mut MPDClient, tracks: &mut HashMap<String, IndexedItem>) {
    // NOTE: assumes playlist naming scheme with hyphen seperators
    let playlists: HashMap<String, IndexedItem> = client
        .send_command("listplaylists\n".to_string())
        .unwrap()
        .split("\n")
        // PERF: is this inefficient/brittle and I should just skip every other element? Yes.
        // Does it work for me to get this to a place I can use it? Also yes.
        .filter(|s| !s.contains("Last-Modified"))
        .filter(|s| !s.is_empty())
        .map(|s| {
            (
                s.replace("-", " ").to_uppercase(),
                IndexedItem {
                    path: "mpc load ".to_string() + s.trim_start_matches("playlist: "),
                    // FIXME: use `config` message here to pull the 'music_directory' or pull from a eurydice
                    // config file
                    cover_path: Some(env::var("HOME").unwrap() + "/Music/playlist-icon.png"),
                    item_type: IndexedItemType::Playlist,
                    title: s.to_string(),
                    artist: "".to_string(),
                },
            )
        })
        .collect();
    tracks.extend(playlists);

    // I used the eurydice to run the eurydice
    tracks.insert(
        "EURYDICE: 3 Random Albums".to_string(),
        IndexedItem {
            path: "eurydice surprise-me album --count 3".to_string(),
            // FIXME: use `config` message here to pull the 'music_directory' or pull from a eurydice
            // config file
            cover_path: Some(env::var("HOME").unwrap() + "/Music/eurydice.png"),
            item_type: IndexedItemType::Playlist,
            title: "Eurydice: 3 Random Albums".to_string(),
            artist: "Eurydice".to_string(),
        },
    );

    tracks.insert(
        "EURYDICE: Random Album".to_string(),
        IndexedItem {
            path: "eurydice surprise-me album".to_string(),
            cover_path: Some(env::var("HOME").unwrap() + "/Music/eurydice.png"),
            item_type: IndexedItemType::Playlist,
            title: "Eurydice: Random Album".to_string(),
            artist: "Eurydice".to_string(),
        },
    );
    tracks.insert(
        "EURYDICE: Mixtape (1 Hour)".to_string(),
        IndexedItem {
            path: "eurydice surprise-me playlist".to_string(),
            cover_path: Some(env::var("HOME").unwrap() + "/Music/eurydice.png"),
            item_type: IndexedItemType::Playlist,
            title: "Eurydice: Mixtape (1 Hour)".to_string(),
            artist: "Eurydice".to_string(),
        },
    );
    tracks.insert(
        "EURYDICE: Mixtape (3 Hours)".to_string(),
        IndexedItem {
            path: "eurydice surprise-me playlist --target-length 180".to_string(),
            cover_path: Some(env::var("HOME").unwrap() + "/Music/eurydice.png"),
            item_type: IndexedItemType::Playlist,
            title: "Eurydice: Mixtape (3 Hours)".to_string(),
            artist: "Eurydice".to_string(),
        },
    );
}

fn parse_info(
    all_track_details: Vec<&str>,
) -> (HashMap<String, IndexedItem>, HashMap<String, IndexedItem>) {
    let mut tracks = HashMap::<String, IndexedItem>::new();
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
        track_key += " - ";
        // FIXME: this will result in multiple entries for the same album when the album has
        // multiple artists (soundtrack composers mostly). Can be fixed with a "Display name"
        // property and keying only off album name.
        album_key += artist;
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
            let cover_path_prefix = Path::new(&(env::var("HOME").unwrap() + "/Music/")).join(album_dir_path);
            for ext in ["cover.jpg", "cover.jpeg", "cover.png"] {
                let path = cover_path_prefix.join(ext);
                if path.exists() {
                    cover_path = Some(path.to_str().unwrap().to_string());
                    break;
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

fn build_summary_table(
    mut tracks: HashMap<String, IndexedItem>,
    albums: HashMap<String, IndexedItem>,
) -> Table {
    tracks.retain(|_, v| v.item_type != IndexedItemType::Playlist);

    // clever fold from https://users.rust-lang.org/t/frequency-of-an-element-in-the-vector/43103/6
    let artist_track_stats = tracks
        .values()
        .fold(HashMap::<String, usize>::new(), |mut m, x| {
            *m.entry(x.artist.clone()).or_default() += 1;
            m
        });
    let artist_album_stats = albums
        .values()
        .fold(HashMap::<String, usize>::new(), |mut m, x| {
            *m.entry(x.artist.clone()).or_default() += 1;
            m
        });

    let mut table_builder = Builder::with_capacity(4, 2);
    [
        vec![
            "Unique Tracks".italic().to_string(),
            tracks.iter().len().to_string().bold().green().to_string(),
        ],
        vec![
            "Unique Albums".italic().to_string(),
            albums.len().to_string().bold().green().to_string(),
        ],
        vec![
            "Unique Artists".italic().to_string(),
            artist_track_stats
                // PERF: avoid this clone
                .clone()
                .into_iter()
                .max_by_key(|(_, v)| *v)
                .map(|(_, v)| v)
                .unwrap()
                .to_string()
                .bold()
                .green()
                .to_string(),
        ],
        vec![
            "Most Tracks By Artist".italic().to_string(),
            artist_track_stats
                .into_iter()
                .sorted_by_key(|x| x.1)
                .rev()
                .take(5)
                .map(|(k, v)| format!("{}: {}", k.italic().red(), v.to_string().bold().green()))
                .join("\n"),
        ],
        vec![
            "Most Albums By Artist".italic().to_string(),
            artist_album_stats
                .into_iter()
                .sorted_by_key(|x| x.1)
                .rev()
                .take(5)
                .map(|(k, v)| format!("{}: {}", k.italic().blue(), v.to_string().bold().green()))
                .join("\n"),
        ],
    ]
    .iter()
    .for_each(|r| table_builder.push_record(r));
    let mut table = table_builder.build();
    table
        .with(tabled::settings::Style::modern_rounded())
        .to_owned()
}
