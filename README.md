# Eurydice
eurydice (named for the [mythical muse](https://en.wikipedia.org/wiki/Eurydice)) is a
small sidecar client to [MPD](https://github.com/MusicPlayerDaemon/MPD) that simply
records track listen history and can be used to generate a random playlist of less
frequently played music, either as a "mixtape" of tracks or as entire albums. 

For all options, run `eurydice --help`.

# Installing
> [!important]
> This *requires* rust 1.88.0 or later to compile due to the use of some `if let`
> constructions. See [issue](https://github.com/rust-lang/rust/issues/53667).

To install, simply clone the repo and run `./install.sh`. This will build eurydice in
release mode and place it in your `~/.local/bin`

## systemd User Unit
A [systemd user unit](https://wiki.archlinux.org/title/Systemd/User) file is included if
you'd like to run eurydice that way. Simply copy this file to your user's systemd config
directory and enable it:

```sh
cp eurydice.service ~/.config/systemd/user/eurydice.service
systemctl --user start eurydice.service # replace with 'enable' if desired
```

# Storage/Backup
eurydice keeps all of it's data in a single sqlite database file, which will be created at
`$XDG_DATA_HOME/.local/share/eurydice/db.db3` if it doesn't already exist. To
backup/snapshot the database, simply copy this file elsewhere, e.g:

```sh
cp $XDG_DATA_HOME/.local/share/eurydice/db.db3 ~/eurydice-db.db3.bak
```

> [!note]
> eurydice follows the [XDG base directory specification](https://specifications.freedesktop.org/basedir-spec/latest/)
> as much as possible. Refer to the [environment variables section](https://specifications.freedesktop.org/basedir-spec/latest/#variables)
> for the fallback paths.

# Caveats
I built eurydice for my own personal use, and as such it's pretty tailored in on how I use
MPD and organize my music library. Feel free to hack on it and submit a PR if you want it
to do something else!

If filing a bug report, please include the logs. eurydice uses `rust-log`'s
[`env_logger`](https://github.com/rust-cli/env_logger) package for this, which you should
set to `trace`:

```sh
RUST_LOG=trace eurydice ...
```

You can also up this level in the systemd unit if desired, and get the logs from the
service with:

```sh
journaltl --user -xeu eurydice.service
```

# Future Work
There's more I want to do with this project, mostly adding `stats` functionality (like
spotify wrapped on steroids) for my listen history. Check the [design file](./DESIGN.md)
for stuff I'll get around to... eventually.
