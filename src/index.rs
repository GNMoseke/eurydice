use std::{ffi::OsStr, path::Path};

use lofty::{self, file::TaggedFileExt, tag::ItemKey};
use log::{debug, warn};
use rusqlite::Connection;
use walkdir::WalkDir;

struct AlbumInfo {
    name: String,
    artist: String,
}

struct TrackInfo {
    title: String,
    artist: String,
}

pub(crate) fn index_collection(music_dir: String, db: &Connection) -> Result<(), rusqlite::Error> {
    // TODO: allow passing of exclude globs here as well so I can skip some subdirs in music.
    // Alternatively, allow them to be filtered out in the list-collection command.
    // or both.

    // recursively walk from root, read all files metadata and store in tracks
    // How do I wanna do album paths? Just go by the directory name metric? e.g. match directory
    // name to album metadata on a track
    // alternatively could just get the albums from the tracks, and then not store a files entry
    // for them but rather when an album is requested via the cli, find all the tracks that are
    // part of it and then simply use the highest quality individual tracks.
    // the problem there is that it does imply ordered titles for the tracks, though playing a
    // whole directory of files already has that same assumption
    // otherwise I have to store that information in the database (or go re-parse all the files
    // when adding to queue)

    WalkDir::new(music_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| !e.path().is_dir())
        .for_each(|entry| {
            println!("\n");
            if let Ok(track_file) = lofty::read_from_path(entry.path()) {
                track_file
                    .primary_tag()
                    .unwrap()
                    .items()
                    .for_each(|f| match f.key() {
                        ItemKey::AlbumTitle => println!("Album Title: {:?}", f.value().text()),
                        ItemKey::TrackTitle => println!("Track Title: {:?}", f.value().text()),
                        ItemKey::Length => println!("Track Length: {:?}", f.value()),
                        _ => println!("Unknown: {:?}", f.value()),
                    });
            } else if entry.path().has_extension(&["png", "jpg", "jpeg"]) {
                warn!("need to handle image {:?}", entry.path())
            } else {
                debug!("Unknown file {:?}, skipping", entry.path())
            }
        });

    Ok(())
}

// https://stackoverflow.com/questions/72392835/check-if-a-file-is-of-a-given-type
trait FileExtension {
    fn has_extension<S: AsRef<str>>(&self, extensions: &[S]) -> bool;
}

impl<P: AsRef<Path>> FileExtension for P {
    fn has_extension<S: AsRef<str>>(&self, extensions: &[S]) -> bool {
        if let Some(extension) = self.as_ref().extension().and_then(OsStr::to_str) {
            return extensions
                .iter()
                .any(|x| x.as_ref().eq_ignore_ascii_case(extension));
        }

        false
    }
}
