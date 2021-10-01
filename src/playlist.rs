//! Keeps track of the contents of and position in the playlist.

use crate::song::Song;
use crate::traits::PathContents;

use glib::ThreadPool;
use itertools::{enumerate, Itertools};
use std::path::PathBuf;
use std::sync::mpsc;

/// Number of worker threads to use for reading song metadata
const N_WORKERS: u32 = 8;

#[derive(Clone)]
/// Keeps track of the contents of and position in the playlist.
pub struct Playlist {
    store: Vec<Song>,
    currently_playing: usize,
}

impl Playlist {
    /// Converts the pathnames in `files` into `Song`s and returns them
    /// as a `Playlist`. If one of `files` cannot be opened as an audio
    /// stream, it is quietly ignored and not added to the playlist.
    ///
    /// Each song is created in a new thread. from() returns when every
    /// thread has finished.
    pub fn from(files: &[PathBuf]) -> Self {
        let (tx, rx) = mpsc::channel();
        let pool = ThreadPool::new_exclusive(N_WORKERS).expect("Failed to create thread pool");

        // Command line arguments are scanned for files, also in
        // subdirectories. The enumerate() is used to keep the order
        // as it was received from the user.
        for (i, path) in enumerate(files)
            .map(|(i, f)| std::iter::repeat(i).zip(f.contents()))
            .flatten()
        {
            let thread_tx = tx.clone();
            pool.push(move || {
                thread_tx
                    .send((i, Song::from(path)))
                    .expect("Failed to send Song to Playlist");
            })
            .expect("Failed to push thread to pool");
        }

        // Close the channel and wait for threads to finish
        drop(tx);
        drop(pool);

        // Sort Songs returned from worker threads based on
        //   1. The original order (i.e. order of command line arguments)
        //   2. Based on the album
        //   3. Based on the track number
        let store: Vec<Song> = rx
            .iter()
            .filter_map(|(i, song)| Some((i, song?)))
            .sorted_by_key(|(i, song)| (*i, song.album_info.to_string(), song.track_number))
            .map(|(_i, song)| song)
            .collect();

        if store.is_empty() {
            eprintln!("No playable files provided\n");
            crate::print_usage_and_exit();
        }

        Self {
            store,
            currently_playing: 0,
        }
    }

    /// Returns the currently playing song on the playlist.
    /// If the playlist is empty, returns None.
    pub fn current(&self) -> Option<&Song> {
        self.store.get(self.currently_playing)
    }

    /// Returns the next song on the playlist and increments
    /// `currently_playing` by 1. If the current song
    /// is the last one on the playlist, returns None.
    pub fn next(&mut self) -> Option<&Song> {
        let song = self.store.get(self.currently_playing + 1)?;
        self.currently_playing += 1;
        Some(song)
    }

    /// Returns the previous song on the playlist and decrements
    /// `currently_playing` by one. If the current song
    /// is the first one on the playlist, returns None.
    pub fn prev(&mut self) -> Option<&Song> {
        self.currently_playing = self.currently_playing.checked_sub(1)?;
        self.store.get(self.currently_playing)
    }

    /// Returns the next song on the playlist without altering
    /// currently playing track. If the current song
    /// is the last one on the playlist, returns None.
    pub fn peek(&self) -> Option<&Song> {
        self.store.get(self.currently_playing + 1)
    }

    /// Returns an iterator over the songs with a boolean indicating
    /// whether the song is currently being played.
    pub fn iter(&self) -> impl Iterator<Item = (bool, &Song)> {
        self.store
            .iter()
            .enumerate()
            .map(move |(i, s)| (i == self.currently_playing, s))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::traits::PathContents;

    use anyhow::Result;
    use rand::seq::SliceRandom;
    use rand::thread_rng;

    fn testcases() -> PathBuf {
        PathBuf::from("testcases/")
    }

    fn args(paths: &[PathBuf]) -> Vec<PathBuf> {
        paths.iter().map(|p| p.canonicalize().unwrap()).collect()
    }

    #[test]
    // soi 1.mp3 2.mp3 3.mp3 etc. should be opened in that order
    fn command_line_arguments_handled_in_order() -> Result<()> {
        gst::init()?;

        let mut files = testcases()
            .canonicalize()?
            .contents()
            .iter()
            .filter(|p| p.extension().unwrap() == "mp3")
            .map(|p| p.to_path_buf())
            .collect::<Vec<PathBuf>>();
        files.shuffle(&mut thread_rng());

        let playlist = Playlist::from(&files);
        let paths = playlist.iter().map(|s| s.1.path.to_path_buf());
        itertools::assert_equal(files, paths);
        Ok(())
    }

    #[test]
    // When argument is a directory, the files within it should be
    // ordered according to the track numbers in the album.
    // In this test case, the files in the directory are alphabetized
    // according to the track numbers.
    fn album_with_ordered_filenames() -> Result<()> {
        gst::init()?;
        let args = args(&[testcases().join("album_with_ordered_filenames")]);

        for (n, item) in Playlist::from(&args).iter().enumerate() {
            let title = format!("Song {}", n + 1);
            assert_eq!(title, item.1.title);
        }

        Ok(())
    }

    #[test]
    // When argument is a directory, the files within it should be
    // ordered according to the track numbers in the album.
    // In this test case, the files in the directory have random names.
    fn album_with_unordered_filenames() -> Result<()> {
        gst::init()?;
        let args = args(&[testcases().join("album_with_unordered_filenames")]);

        for (n, item) in Playlist::from(&args).iter().enumerate() {
            let title = format!("Song {}", n + 1);
            assert_eq!(title, item.1.title);
        }

        Ok(())
    }

    #[test]
    // When argument is a directory, the files within it should be
    // ordered according to the track numbers in the album.
    // In this test case, the files in the directory have random names
    // and random creation times.
    fn album_with_random_ctime() -> Result<()> {
        gst::init()?;
        let args = args(&[testcases().join("album_with_random_ctime")]);

        for (n, item) in Playlist::from(&args).iter().enumerate() {
            let title = format!("Song {}", n + 1);
            assert_eq!(title, item.1.title);
        }
        Ok(())
    }

    #[test]
    // In album_with_random_ctime test case, there is a small possibility
    // that the random filenames are alphabetically sorted just Song 1,
    // Song 2, Song 3 etc. This ensures at least one of the files is
    // out of order and the files are suitable for the test case.
    fn album_with_random_ctime_not_alphabetical() {
        gst::init().unwrap();
        let args = args(&[testcases().join("album_with_random_ctime")]);

        for (i, x) in args[0].contents().iter().enumerate() {
            let song = Song::from(x.to_path_buf()).unwrap();
            if song.track_number != i as u32 {
                return;
            }
        }
        panic!("Re-run testcases/generate.sh");
    }

    #[test]
    // When a directory is passed as an argument and the directory
    // contains more than one subdirectory, the subdirs should be
    // played in order (i.e. not start with track 1 from every album,
    // go on to the track 2s etc.).
    fn two_album_with_ordered_filenames() -> Result<()> {
        gst::init()?;
        let args = args(&[testcases()]);

        let playlist = Playlist::from(&args);
        let mut song = playlist.iter();
        assert_eq!(1, song.next().unwrap().1.track_number);
        assert_eq!(2, song.next().unwrap().1.track_number);
        assert_eq!(3, song.next().unwrap().1.track_number);
        assert_eq!(4, song.next().unwrap().1.track_number);
        Ok(())
    }
}
