use anyhow::{Context, Result};
use std::env::Args;
use std::path::PathBuf;

pub trait ArgFiles {
    fn files(self) -> Result<Vec<PathBuf>>;
}

/// Ensures all command line arguments are canonical absolute paths
impl ArgFiles for Args {
    fn files(self) -> Result<Vec<PathBuf>> {
        self.skip(1)
            .map(|path| std::fs::canonicalize(&path).context(path))
            .collect()
    }
}
