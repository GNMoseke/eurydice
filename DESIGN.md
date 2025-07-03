# CLI
* `surpriseme [album | playlist]`: Generate and start a playlist of songs that have not
had as many plays. If `album` is provided, do the same but with an album (or multiple
depending on target length)
    * `--target-length [50min]`: Flag for the target amount of time for the created
    playlist. If not given, default to either one album or 10 songs.
    * `--sameartist`: enforce that anything chosen has to be from the same artist (no-op
    if `album` option is given)
    * `--interactive`: prompt with the playlist generated, ask to
    confirm/regenerate/cancel
* `stats`: More stats breakdown about most played tracks, artists, albums, etc in a small
  table
* `daemon`: start the daemon half

# SQL Schema
`tracks` table:

```
id | title | artist | album | length (s) | plays
```

`history` table:
```
time | song_id (fk->tracks)
```



# TODO
- [ ] Daemon half
    - [ ] monitors mpd and writes stats on song update
    - [ ] creates sqlite file if none exist
    - [ ] add systemd file
- [ ] Command half
    - [ ] Queries for appropriate combo of least played
    - [ ] creates playlist
    - [ ] publishes to MPD

