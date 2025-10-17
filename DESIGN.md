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
* `collection`: info about music collection (basically wrapper for MPD commands)
    * `--ouput [summary (default) | rofi | detailed]`

# SQL Schema
`tracks` table:

```
id | title | artist | album | lengthseconds | playcount | path
```

`history` table:
```
time | song_id (fk->tracks)
```

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
    - [ ] Better error handling
    - [!] Add config (ignore certain music subdirs in daemon mode, etc)
    - [ ] [shell completions](https://docs.rs/clap_complete/latest/clap_complete/)
- [x] Collection stats
- [x] Profile the performance of `collection`. I suspect the glob call and the mpc round
trip are the most expensive pieces.
    * It was indeed the glob I am so smart
- [ ] `neverplayed` to compare what's in collection with what's in play history
