# CLI
* `surpriseme [album | playlist]`: Generate and start a playlist of songs that have not
had as many plays. If `album` is provided, do the same but with an album (or multiple
depending on target length)
    * `--target-length [50]`: Flag for the target amount of time for the created
    playlist. If not given, default to either one album or one hour of songs.
    * `--sameartist`: enforce that anything chosen has to be from the same artist (no-op
    if `album` option is given)
* `stats`: More stats breakdown about most played tracks, artists, albums, etc in a small
  table
* `daemon`: start the daemon half

# Indexing
```sh
eurydice index --music-dir [optional path]
eurydice list-collection --output summary # default
eurydice list-collection --output rofi
eurydice list-collection --output detailed

# might need this command to leverage with rofi - basically just passes through to MPC
eurydice queue album [foo]
eurydice queue track [bar]
eurydice queue playlist [baz]
```

## TODO
- [ ] How to handle playlists?
- [ ] Should the daemon intelligently re-index is the background when the music subdir
changes?



# TODO
- [x] Daemon half
    - [x] monitors mpd and writes stats on song update
    - [x] creates sqlite file if none exist
    - [x] add systemd file
- [x] Command half
    - [x] Queries for appropriate combo of least played
    - [x] creates playlist
    - [x] publishes to MPD
- [>] stats (Eternal WIP)
- [ ] General
    - [x] Logging
    - [x] Better error handling
    - [ ] Add config (ignore certain music subdirs in daemon mode, set music dir, etc)
    - [ ] [shell completions](https://docs.rs/clap_complete/latest/clap_complete/)
- [ ] Quick way to add a song to a "favorites" playlist (could also do this in my eww
widget)
