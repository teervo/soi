//! A single audio track on the playlist.

use crate::traits::{AudioPlaybin, PathToURI};

use anyhow::Result;
use gst::prelude::*;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// The `Song` object is a single audio track on the
/// [`Playlist`][crate::playlist::Playlist].
///
/// It stores the URI, metadata and duration of the audio file.
#[derive(Clone, Default)]
pub struct Song {
    /// Path to the audio file
    pub path: PathBuf,

    pub album_artist: String,
    pub album_title: String,
    pub album_info: String, // "Artist: Title (Year)"

    pub artist: String,
    pub title: String,
    pub track_number: u32,
    year: Option<i32>,

    pub duration: Duration,
}

impl Song {
    /// Creates a new `Song` from the provided `PathBuf`
    pub fn from(path: PathBuf) -> Option<Self> {
        let playbin = Self::setup_pipeline().ok()?;

        let (duration, tags) = Self::get_track_info(&path, playbin)?;

        let mut song = Self {
            path,
            duration,
            ..Self::default()
        };
        song.read_metadata(&tags);

        Some(song)
    }

    /// Creates and sets up the GStreamer pipeline to verify
    /// the input files and extract the metadata.
    fn setup_pipeline() -> Result<gst::Element> {
        // This is all a bit ugly. It does two things,
        // check whether this is an audio file we can
        // add to the playlist, and add any relevant tags
        // to the Song struct.
        let playbin =
            gst::ElementFactory::make("playbin", None).expect("setup_pipeline(): playbin");
        let sink = gst::ElementFactory::make("fakesink", None).expect("setup_pipeline(): fakesink");
        playbin.set_property("audio-sink", sink)?;
        playbin.disable_video()?;

        Ok(playbin)
    }

    /// Decodes the audio file until we have the duration and tags
    /// read. On error, returns None.
    fn get_track_info(path: &Path, playbin: gst::Element) -> Option<(Duration, gst::TagList)> {
        let mut duration = None;
        let mut tags = None;

        playbin
            .set_property("uri", path.to_uri())
            .expect("Unable to set pipeline URI");
        playbin
            .set_state(gst::State::Playing)
            .expect("Unable to set the pipeline to the `Playing` state");

        // Decode file until the tags and the duration are read.
        // In case of an error (not an audio file or a corrupt one),
        // stop and return None.
        for msg in playbin.bus()?.iter_timed(gst::ClockTime::NONE) {
            match msg.view() {
                gst::MessageView::Tag(msg) => {
                    tags = Some(msg.tags());
                }
                gst::MessageView::Error(e) => {
                    glib::g_debug!("song", "{:?}: {}", path, e.error());
                    break;
                }
                _ => (),
            }

            // When duration can be read from an audio file seems to vary
            // a lot depending on file format etc. We just keep trying.
            if duration.is_none() {
                duration = playbin
                    .query_duration::<gst::format::Time>()
                    .map(|ct| ct.into());
            }

            if duration.and(tags.as_ref()).is_some() {
                break;
            }
        }

        // Clean up
        playbin
            .set_state(gst::State::Null)
            .expect("Unable to set the pipeline to the `Null` state");

        Some((duration?, tags?))
    }

    /// Populates the `Song`s metadata information from
    /// the provided `TagList`.
    fn read_metadata(&mut self, tags: &gst::TagList) {
        self.album_title = match tags.get::<gst::tags::Album>() {
            Some(album) => album.get().to_string(),
            None => "Unknown album".to_string(),
        };

        self.artist = match tags.get::<gst::tags::Artist>() {
            Some(artist) => artist.get().to_string(),
            None => "Unknown artist".to_string(),
        };

        self.album_artist = match tags.get::<gst::tags::AlbumArtist>() {
            Some(artist) => artist.get().to_string(),
            None => self.artist.to_string(),
        };

        // If title is not found, fallback to basename
        self.title = match tags.get::<gst::tags::Title>() {
            Some(title) => title.get().to_string(),
            None => format!("{:?}", self.path.file_stem().unwrap_or_default())
                .trim_matches('"')
                .to_string(),
        };

        self.track_number = tags
            .get::<gst::tags::TrackNumber>()
            .map(|v| v.get())
            .unwrap_or_default();

        self.year = tags.get::<gst::tags::DateTime>().map(|v| v.get().year());

        self.album_info = match self.year {
            Some(year) => format!("{}: {} ({})", self.album_artist, self.album_title, year),
            None => format!("{}: {}", self.album_artist, self.album_title),
        };
    }

    /// Returns true when album is not released by a single artist
    pub fn part_of_compilation(&self) -> bool {
        self.album_artist == "Various Artists"
    }
}

impl std::fmt::Display for Song {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // If the album is a compilation, we also print
        // the artist. Otherwise, just the song title.
        let mut title = if self.part_of_compilation() {
            format!("{}: {}", self.artist, self.title)
        } else {
            self.title.to_string()
        };

        // If width is specified and smaller than title length,
        // we truncate the title.
        if let Some(width) = f.width() {
            if width < title.len() {
                title.truncate(width - 1);
                title.push('…');
            }

            title.fmt(f)
        } else {
            title.fmt(f)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    // If an audio file has no tags, the basename should be used as title
    fn basename_used_for_tagless_files() {
        gst::init().unwrap();
        let path = PathBuf::from("testcases/album_with_no_tags/1. Song 1.mp3")
            .canonicalize()
            .unwrap();
        let song = Song::from(path).unwrap();
        assert_eq!(song.title, "1. Song 1");
    }
}
