//! Handles terminal output.

use crate::backend::BackendState;
use crate::playlist::Playlist;
use crate::song::Song;
use crate::traits::PrettyDuration;

use anyhow::Result;
use itertools::Itertools;
use std::convert::TryFrom;
use std::io::{stdout, Write};
use termion::color;
use termion::raw::{IntoRawMode, RawTerminal};

/// Stores the state of the terminal and handles cleanup when dropped.
/// (although drop() is not really guaranteed to be called on exit...)
pub struct Output {
    stdout: RawTerminal<std::io::Stdout>,
    lines_printed: usize, // Number of lines printed on last refresh
    display_help: bool,   // Whether help mode is on
}

impl Output {
    /// Sets terminal into raw mode and returns a new `Output` struct
    pub fn new() -> Self {
        Self {
            stdout: stdout().into_raw_mode().expect("Unable to open stdout"),
            lines_printed: 0,
            display_help: false,
        }
    }

    /// Restores the state of the terminal before quitting
    pub fn cleanup(&self) {
        println!();
        self.stdout.suspend_raw_mode().ok();
    }

    /// FIXME add comment
    pub fn toggle_help(&mut self) -> Result<()> {
        self.display_help = match self.display_help {
            true => false,
            false => true,
        };

        Ok(())
    }

    /// Refreshes the output printed to the user.
    pub fn refresh(&mut self, state: BackendState, playlist: &Playlist) -> Result<()> {
        // Move cursor back up to where we start printing
        // https://vt100.net/docs/vt100-ug/chapter3.html#CUU
        if self.lines_printed > 0 {
            self.stdout
                .write_all(format!("\x1b[{}A", self.lines_printed).as_ref())?;
        }

        let output = match self.display_help {
            false => Self::generate_output(state, playlist)?,
            true => Self::generate_help()?,
        };

        self.stdout.write_all(output.join("\r\n").as_ref())?;
        self.stdout.write_all(b"\r")?;

        // -1 because last line has no newline:
        self.lines_printed = output.len() - 1;

        self.stdout.flush()?;
        Ok(())
    }

    /// Returns the lines to be printed to the terminal.
    ///
    /// If the whole playlist does not fit into the terminal, the lines
    /// are printed so that the currently played song is in the middle
    /// of the window.
    fn generate_output(state: BackendState, playlist: &Playlist) -> Result<Vec<String>> {
        let mut ret = Vec::new();
        let mut center: usize = 0; // Index of the currently played song

        let (terminal_height, terminal_width) = {
            let (w, h) = termion::terminal_size()?;
            (usize::try_from(h)?, usize::try_from(w)?)
        };

        // TODO: This group_by() is being ran every 100ms or so
        // It might be better to store the songs grouped by album
        // in the playlist instead. This would, however, make Playlist
        // more complex.
        for (album, songs) in &playlist
            .iter()
            .group_by(|(_, song)| song.album_info.to_string())
        {
            ret.push(format!(
                "{}{:>width$}{}",
                termion::style::Underline,
                album,
                termion::style::Reset,
                width = terminal_width,
            ));

            for (playing, song) in songs {
                if playing {
                    center = ret.len();
                    ret.push(Self::format_playing_song(song, &state, terminal_width));
                } else {
                    ret.push(Self::format_song(song, terminal_width));
                }
            }
        }

        // Determine which part of the output to print for it to fit
        // the screen and for the currently playing song to be visible
        if ret.len() <= terminal_height {
            Ok(ret)
        } else if ret.len() - center < terminal_height / 2 {
            let start_index = ret.len() - terminal_height;
            Ok(ret[start_index..].to_vec())
        } else {
            let start_index = center.saturating_sub(terminal_height / 2);
            let end_index = std::cmp::min(ret.len(), start_index + terminal_height);
            Ok(ret[start_index..end_index].to_vec())
        }
    }

    /// FIXME
    fn generate_help() -> Result<Vec<String>> {
        let mut ret = Vec::new();
        let (terminal_height, _terminal_width) = {
            let (w, h) = termion::terminal_size()?;
            (usize::try_from(h)?, usize::try_from(w)?)
        };

        let empty_line = termion::clear::AfterCursor.to_string();

        ret.push(empty_line.clone());
        ret.push(format!("Keyboard shortcuts{}", termion::clear::AfterCursor));
        ret.push(empty_line.clone());

        ret.push(format!(
            " k or up arrow     previous song{}",
            termion::clear::AfterCursor
        ));

        ret.push(format!(
            " j or down arrow   next song{}",
            termion::clear::AfterCursor
        ));

        ret.push(format!(
            " h or left arrow   seek backwards{}",
            termion::clear::AfterCursor
        ));

        ret.push(format!(
            " l or right arrow  seek forward{}",
            termion::clear::AfterCursor
        ));

        while ret.len() < terminal_height {
            ret.push(empty_line.clone());
        }

        Ok(ret)
    }

    /// Returns the line of output to be printed
    /// for the currently playing song.
    fn format_playing_song(song: &Song, state: &BackendState, terminal_width: usize) -> String {
        let icon = match (state.playing, state.muted) {
            (true, true) => "🔇",
            (true, false) => " ▶",
            (false, _) => " ⏸",
        };

        let time = format!("{}/{}", state.position.pretty(), song.duration.pretty());

        format!(
            "{} {}{:>3} {:width$} {:>time_width$}{}",
            color::Fg(color::LightWhite),
            icon,
            song.track_number,
            song,
            time,
            color::Fg(color::Reset),
            width = terminal_width - 9 - time.len(),
            time_width = time.len() + 1
        )
    }

    /// Returns the line of output to be printed for a song that is not
    /// being played.
    fn format_song(song: &Song, terminal_width: usize) -> String {
        let duration = song.duration.pretty();
        format!(
            "{}{:>6} {:width$} {:>time_width$}{}",
            color::Fg(color::White),
            song.track_number,
            song,
            duration,
            color::Fg(color::Reset),
            width = terminal_width - 9 - duration.len(),
            time_width = duration.len() + 1
        )
    }
}
