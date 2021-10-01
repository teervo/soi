//! Reads and interprets user key presses.

use termion::event::Key;
use termion::input::TermRead;

/// Valid user actions the main program needs to act on.
pub enum UserInput {
    Mute,
    Pause,
    Stop,
    Next,
    Prev,
    SeekBackward,
    SeekForward,
}

/// Interprets user key presses as `UserInput` variants.
pub fn handle_user_input() -> Option<UserInput> {
    match read_key_press()? {
        Key::Char('m') => Some(UserInput::Mute),
        Key::Char(' ') => Some(UserInput::Pause),
        Key::Char('q') => Some(UserInput::Stop),
        Key::Char('h') | Key::Left => Some(UserInput::SeekBackward),
        Key::Char('j') | Key::Down => Some(UserInput::Next),
        Key::Char('k') | Key::Up => Some(UserInput::Prev),
        Key::Char('l') | Key::Right => Some(UserInput::SeekForward),
        _ => None,
    }
}

/// Reads single key press from stdin(), returning None if no input
/// is available.
fn read_key_press() -> Option<Key> {
    std::io::stdin().lock().keys().next()?.ok()
}
