//! Communicates with the audio playback engine.

use crate::song::Song;
use crate::traits::{AudioPlaybin, PathToURI, UnwrappedMutex};

use anyhow::Result;
use glib::{source::Priority, MainContext};
use gst::prelude::*;
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Clone)]
/// The internal state of the playback backend
pub struct Backend {
    playbin: gst::Element,
    next_uri: Arc<Mutex<Option<String>>>,
    main_tx: glib::Sender<BackendMessage>,
}

/// State of playback
pub struct BackendState {
    pub position: std::time::Duration,
    pub playing: bool,
    pub muted: bool,
}

/// Message from `Backend` to the main application
pub enum BackendMessage {
    ReachedEndOfSong,
    ReachedEndOfPlaylist,
    RequestNextSong,
    State(BackendState),
}

impl Backend {
    /// Initializes the GStreamer backend and sets up signal
    /// handling. Returns a tupe with the new Backend object and
    /// a Receiver for communication during playback.
    pub fn new() -> (Self, glib::Receiver<BackendMessage>) {
        gst::init().expect("Unable to initialize GStreamer");
        let playbin = gst::ElementFactory::make("playbin", None)
            .expect("Unable to create the `playbin` element");
        playbin.disable_video().ok(); // .ok() to ignore any errors

        // Asynchronous channel to communicate with main() with
        let (main_tx, main_rx) = MainContext::channel(Priority::default());
        // Handle messages from GSTreamer bus
        playbin
            .bus()
            .expect("Failed to get GStreamer message bus")
            .add_watch(glib::clone!(@strong main_tx => move |_bus, msg| {
                match msg.view() {
                    gst::MessageView::Eos(_) =>
                        main_tx.send(BackendMessage::ReachedEndOfPlaylist)
                        .expect("Unable to send message to main()"),
                    gst::MessageView::Error(e) =>
                        glib::g_debug!("song", "{}", e.error()),
                        _ => (),
                }
                glib::Continue(true)
            }))
            .expect("Failed to connect to GStreamer message bus");

        let this = Self {
            playbin,
            next_uri: Arc::new(Mutex::new(None)),
            main_tx,
        };

        // Switch to next song when reaching end of current track
        this.playbin
            .connect(
                "about-to-finish",
                false,
                glib::clone!(@strong this => move |_args| {
                   this.dequeue();
                   None
                }),
            )
            .expect("Failed to connect playbin's `about-to-finish` signal");

        // Update main() with backend state every 100ms
        glib::source::timeout_add(
            Duration::from_millis(100),
            glib::clone!(@strong this => move || {
               this.main_tx.send(BackendMessage::State(this.state()))
                   .expect("Unable to send message to main()");
            glib::Continue(true)
            }),
        );

        (this, main_rx)
    }

    /// Returns true if the stream is not currently paused
    pub fn playing(&self) -> bool {
        self.playbin.current_state() != gst::State::Paused
    }

    /// Returns true if the application is currently muted
    pub fn muted(&self) -> bool {
        if let Ok(prop) = self.playbin.property("mute") {
            prop.get().unwrap_or(false)
        } else {
            false
        }
    }

    /// Starts playback of `song`. If `song` is None, does nothing.
    pub fn play(&self, song: Option<&Song>) -> Result<()> {
        if let Some(song) = song {
            self.playbin.set_state(gst::State::Ready)?;
            self.playbin.set_property("uri", song.path.to_uri())?;
            self.playbin.set_state(gst::State::Playing)?;
            self.main_tx
                .send(BackendMessage::RequestNextSong)
                .expect("Unable to send message to main()");
        }

        Ok(())
    }

    /// Stops playback to quit program.
    pub fn stop(&self) -> Result<()> {
        self.playbin.set_state(gst::State::Null)?;
        self.main_tx
            .send(BackendMessage::ReachedEndOfPlaylist)
            .expect("Unable to send message to main()");
        Ok(())
    }

    /// Mutes/unmutes playback
    pub fn toggle_mute(&self) -> Result<()> {
        let muted: bool = self.playbin.property("mute")?.get()?;
        self.playbin.set_property("mute", !muted)?;
        Ok(())
    }

    /// Pauses/unpauses playback
    pub fn toggle_pause(&self) -> Result<()> {
        match self.playbin.current_state() {
            gst::State::Playing => self.playbin.set_state(gst::State::Paused),
            _ => self.playbin.set_state(gst::State::Playing),
        }?;
        Ok(())
    }

    /// Returns the current position in the played track
    pub fn position(&self) -> std::time::Duration {
        self.playbin
            .query_position::<gst::ClockTime>()
            .unwrap_or_default()
            .into()
    }

    /// Returns the state of the backend as a BackendState struct
    pub fn state(&self) -> BackendState {
        BackendState {
            position: self.position(),
            playing: self.playing(),
            muted: self.muted(),
        }
    }

    /// Sets the song to be played after the end of the current one
    /// is reached. This is necessary for gapless playback.
    pub fn enqueue(&mut self, song: Option<&Song>) {
        *self.next_uri.lockk() = song.map(|s| s.path.to_uri());
    }

    /// Sets the playbin URI to `self.next_uri`, when it is not None.
    /// This function is to be used from GStreamer playbin's
    /// about-to-finish callback only.
    pub fn dequeue(&self) {
        if let Some(uri) = &*self.next_uri.lockk() {
            self.playbin
                .set_property("uri", uri)
                .expect("Unable to set playbin URI");
            self.main_tx
                .send(BackendMessage::ReachedEndOfSong)
                .expect("Unable to send message to main()");
            self.main_tx
                .send(BackendMessage::RequestNextSong)
                .expect("Unable to send message to main()");
        }
    }

    /// Skips forward 5 seconds
    pub fn seek_forward(&self) -> Result<()> {
        if let Some(t) = self.playbin.query_position::<gst::ClockTime>() {
            self.seek_to(t + gst::ClockTime::from_seconds(5));
        }

        Ok(())
    }

    /// Skips backward 5 seconds
    pub fn seek_backward(&self) -> Result<()> {
        if let Some(t) = self.playbin.query_position::<gst::ClockTime>() {
            let pos = t.saturating_sub(gst::ClockTime::from_seconds(5));
            self.seek_to(pos);
        }

        Ok(())
    }

    /// Seeks to the specified position in the current song
    fn seek_to(&self, pos: gst::ClockTime) {
        self.playbin.seek_simple(gst::SeekFlags::FLUSH, pos).ok(); // ignore any errors
    }
}

impl Drop for Backend {
    /// Cleans up GStreamer pipeline when `Backend` is dropped.
    fn drop(&mut self) {
        self.playbin
            .set_state(gst::State::Null)
            .expect("Unable to set the pipeline to the `Null` state");
    }
}
