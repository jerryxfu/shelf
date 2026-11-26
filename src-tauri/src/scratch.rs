//! For tests only: a folder of its own for one test, removed when the test ends.

use std::path::{Path, PathBuf};

pub struct Scratch(PathBuf);

impl Scratch {
    /// `test` goes into the folder's name, so that tests running at the same time each get their own.
    pub fn new(test: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("shelf-test-{test}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }

    pub fn dir(&self) -> &Path {
        &self.0
    }

    /// Writes a file in the folder and gives its path.
    pub fn file(&self, name: &str, contents: &str) -> PathBuf {
        let path = self.0.join(name);
        std::fs::write(&path, contents).unwrap();
        path
    }
}

/// `Drop` is what Rust runs when a value goes away: here, at the end of the test, passed or failed.
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
