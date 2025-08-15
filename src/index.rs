pub(crate) fn index_collection(music_dir_path: &std::path::Path) -> Result<(), rusqlite::Error> {
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
    todo!();
}
