use itertools::Itertools;
use log::{debug, error, trace};
use std::env;
use std::io::{BufReader, prelude::*};
use std::os::unix::net::UnixStream;

use crate::surprise_me;

pub(crate) struct MPDClient {
    stream: UnixStream,
    reader: BufReader<UnixStream>,
}

impl MPDClient {
    pub(crate) fn connect() -> MPDClient {
        debug!("Initializing MPD connection");
        // NOTE: MUST use a unix socket to manage the queue locally. This is "documented" in the mpd
        // protocal manual here: https://mpd.readthedocs.io/en/latest/client.html#introduction
        // where "local socket" means "unix socket".
        // See also: https://github.com/MusicPlayerDaemon/MPD/issues/2184
        let stream = UnixStream::connect(
            env::var("XDG_RUNTIME_DIR").unwrap_or("/run".to_string()) + "/mpd/socket",
        )
        .unwrap_or_else(|err| panic!("Failed to connect to MPD socket: {err:?}"));

        let mut reader = BufReader::new(stream.try_clone().expect("MPD connection invalid"));
        let recv: Vec<u8> = reader
            .fill_buf()
            .expect("MPD connection returned initial handshake")
            .to_vec();
        reader.consume(recv.len());
        let connect_ack = String::from_utf8(recv).expect("MPD connection handshake readable");

        // NOTE: Protocol version agnostic here, see:
        // https://mpd.readthedocs.io/en/latest/protocol.html#protocol-overview
        if !connect_ack.contains("OK MPD") {
            panic!("Unknown connection string: {connect_ack}")
        }

        MPDClient { stream, reader }
    }

    pub(crate) fn add_to_queue(&mut self, tracks: &[surprise_me::SelectedTrack]) {
        debug!("Adding {} tracks to queue", tracks.len());
        trace!("Adding {}", tracks.iter().map(|t| t.path.clone()).join(","));
        let command = "command_list_begin\n".to_owned()
            + &tracks
                .iter()
                .map(|t| "add \"".to_string() + &t.path + "\"")
                .join("\n")
            + "\n"
            + "status\n"
            + "command_list_end\n";
        match self.send_command(command) {
            Some(status) => {
                // Decide what to do based on player state after adding to the queue
                // nothing in queue and eurydice is run: state == stop -> send play
                // something is playing and eurydice is run: state == play -> do nothing
                // something is paused and eurydice is run: state == pause -> do nothing
                // TODO: heavy handed split sequence, I could use a regex split to a hashmap for the rest of
                // the status info, but don't need it right now
                let player_state = status
                    .split_once("state: ")
                    .unwrap()
                    .1
                    .split_once("\n")
                    .unwrap()
                    .0;
                debug!("Player in state {player_state}");

                if player_state == "stop" {
                    self.send_command("play 0\n".to_string());
                }
            }
            None => panic!("MPD status returned no information, cannot manage queue"),
        }
    }

    pub(crate) fn send_command(&mut self, command: String) -> Option<String> {
        debug!("Sending MPD command {}", command.trim());
        self.stream.write_all(command.as_bytes()).ok()?;
        self.stream.flush().ok()?;

        let mut full_msg = String::new();
        loop {
            let mut curr_line = String::new();
            _ = self.reader.read_line(&mut curr_line);
            match curr_line {
                val if val == "OK\n" => {
                    trace!("Response for {command} was OK");
                    break;
                }
                // FIXME: regex for proper ACK [error@command_listNum] otherwise this matches
                // things with the word BLACK in all caps for example
                val if val.ends_with("ACK [") => {
                    error!("Failed response for {command}: {val}");
                    break;
                }
                val => full_msg += &val,
            };
        }
        Some(full_msg)
    }
}
