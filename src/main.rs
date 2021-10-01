//! A music player for the pre-streaming era.

mod backend;
mod input;
mod output;
mod playlist;
mod song;
mod traits;

use backend::BackendMessage;
use input::{handle_user_input, UserInput};
use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::time::Duration;
use traits::{ArgFiles, UnwrappedMutex};

fn main() -> anyhow::Result<()> {
    handle_cmd_line_flags();

    let ctx = glib::MainContext::default();
    let _guard = ctx.acquire();
    let mainloop = glib::MainLoop::new(Some(&ctx), false);

    let (mut backend, backend_rx) = backend::Backend::new();
    let playlist = Arc::new(Mutex::new(playlist::Playlist::from(
        &std::env::args().files()?,
    )));
    let mut output = output::Output::new();

    // New thread for waiting for user input
    let (input_tx, input_rx) = glib::MainContext::channel(glib::source::Priority::default());
    std::thread::spawn(move || loop {
        match handle_user_input() {
            None => sleep(Duration::from_millis(100)),
            Some(x) => input_tx
                .send(x)
                .expect("Failed to send user input to main thread"),
        }
    });

    // Send user input to backend
    input_rx.attach(
        None,
        glib::clone!(@strong backend, @strong playlist, @strong mainloop => move |msg| {
            match msg {
                UserInput::Mute => backend.toggle_mute(),
                UserInput::Pause => backend.toggle_pause(),
                UserInput::Stop => backend.stop(),
                UserInput::Next => backend.play(playlist.lockk().next()),
                UserInput::Prev => backend.play(playlist.lockk().prev()),
                UserInput::SeekBackward => backend.seek_backward(),
                UserInput::SeekForward => backend.seek_forward(),
             }.expect("Error while handling user input");
            glib::Continue(true)
        }),
    );

    // Start playback
    backend.play(playlist.lockk().current())?;

    // Handle messages from backend
    backend_rx.attach(
        None,
        glib::clone!(@strong mainloop => move |msg| {
            match msg {
                BackendMessage::ReachedEndOfSong => {
                    // Backend switches to the next track itself,
                    // we just need to notify playlist about the change.
                    playlist.lockk().next();
                }
                BackendMessage::ReachedEndOfPlaylist => {
                    output.cleanup();
                    mainloop.quit();
                }
                BackendMessage::RequestNextSong => {
                    backend.enqueue(playlist.lockk().peek());
                }
                BackendMessage::State(state) => {
                    output.refresh(state, &playlist.lockk())
                        .ok(); // ignore any output errors
                }
            };
            glib::Continue(true)
        }),
    );

    mainloop.run();
    Ok(())
}

pub fn print_version_and_exit() {
    println!("soi {}", env!("CARGO_PKG_VERSION"));
    std::process::exit(1);
}

pub fn print_usage_and_exit() {
    eprintln!("Usage: soi FILES...\n");

    eprintln!("      --help                   Show this help message");
    eprintln!("      --version                Display version information");

    std::process::exit(1);
}

fn handle_cmd_line_flags() {
    for flag in std::env::args().filter(|x| x.starts_with('-')) {
        match flag.as_str() {
            "--help" => print_usage_and_exit(),
            "--version" => print_version_and_exit(),
            x => {
                eprintln!("Unknown option {}", x);
                print_usage_and_exit();
            }
        }
    }
}
