use std::path::PathBuf;

/// This trait allows for retrieval of all files within an
/// object representing a directory, e.g. `Path` or `PathBuf`.
pub trait PathContents {
    fn contents(&self) -> Vec<PathBuf>;
}

impl PathContents for PathBuf {
    /// Returns a vector of the files within `path`, descending into
    /// subdirectories. If `path` is a file, it will be the only item
    /// in the vector.
    fn contents(&self) -> Vec<PathBuf> {
        if self.is_file() {
            vec![self.to_path_buf()]
        } else if self.is_dir() {
            self.read_dir()
                .unwrap_or_else(|_| panic!("Unable to open {:?}", self))
                .filter_map(|entry| entry.map(|f| f.path()).ok())
                .map(|pb| pb.contents())
                .flatten()
                .collect()
        } else {
            vec![]
        }
    }
}
