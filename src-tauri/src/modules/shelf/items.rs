//! The shelf itself: the files held until the user acts on them.
//!
//! Plain Rust. Nothing here knows about windows, the tray or Tauri, so the interface can change without touching it.
//! The shelf keeps paths, never file contents, and it never changes anything on disk.

use std::path::{Path, PathBuf};

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShelfItem {
    pub id: u64,
    pub path: PathBuf,
    pub name: String,
    /// Size in bytes. `None` for a folder: adding up a folder's contents can be slow, and nothing needs it yet.
    pub size: Option<u64>,
    pub is_dir: bool,
}

#[derive(Debug, Default)]
pub struct Shelf {
    items: Vec<ShelfItem>,
    next_id: u64,
}

impl Shelf {
    pub fn items(&self) -> &[ShelfItem] {
        &self.items
    }

    /// Puts files on the shelf, in the order given. A path that is already there, or that cannot be read, is skipped.
    pub fn add<P: AsRef<Path>>(&mut self, paths: impl IntoIterator<Item = P>) {
        for path in paths {
            let path = path.as_ref();
            if self.items.iter().any(|item| item.path == path) {
                continue;
            }
            if let Some(details) = Details::read(path) {
                self.next_id += 1;
                self.items.push(ShelfItem {
                    id: self.next_id,
                    path: path.to_path_buf(),
                    name: details.name,
                    size: details.size,
                    is_dir: details.is_dir,
                });
            }
        }
    }

    /// Takes an item off the shelf. The file itself is left alone.
    pub fn remove(&mut self, id: u64) {
        self.items.retain(|item| item.id != id);
    }

    /// Reads every file's details again, and drops the items whose file is gone (moved or deleted somewhere else).
    pub fn refresh(&mut self) {
        self.items.retain_mut(|item| match Details::read(&item.path) {
            Some(details) => {
                item.size = details.size;
                item.is_dir = details.is_dir;
                true
            }
            None => false,
        });
    }
}

struct Details {
    name: String,
    size: Option<u64>,
    is_dir: bool,
}

impl Details {
    /// `None` when the file does not exist, cannot be read, or has a name or path that is not valid Unicode (the
    /// interface could not show it or hand it back).
    fn read(path: &Path) -> Option<Self> {
        path.to_str()?;
        let name = path.file_name()?.to_str()?.to_owned();
        let metadata = std::fs::metadata(path).ok()?;
        let is_dir = metadata.is_dir();
        Some(Self {
            name,
            size: (!is_dir).then_some(metadata.len()),
            is_dir,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scratch::Scratch;

    #[test]
    fn adds_files_and_folders_with_their_details() {
        let scratch = Scratch::new("details");
        let file = scratch.file("notes.txt", "hello");
        let mut shelf = Shelf::default();

        shelf.add([file.as_path(), scratch.dir()]);

        let items = shelf.items();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].name, "notes.txt");
        assert_eq!(items[0].size, Some(5));
        assert!(!items[0].is_dir);
        assert!(items[1].is_dir);
        assert_eq!(items[1].size, None);
    }

    #[test]
    fn skips_a_path_already_on_the_shelf_and_one_that_does_not_exist() {
        let scratch = Scratch::new("skips");
        let file = scratch.file("a.txt", "a");
        let mut shelf = Shelf::default();

        shelf.add([&file]);
        shelf.add([file.clone(), scratch.dir().join("missing.txt")]);

        assert_eq!(shelf.items().len(), 1);
    }

    #[test]
    fn removing_an_item_leaves_the_file_on_disk() {
        let scratch = Scratch::new("remove");
        let file = scratch.file("keep.txt", "still here");
        let mut shelf = Shelf::default();
        shelf.add([&file]);

        shelf.remove(shelf.items()[0].id);

        assert!(shelf.items().is_empty());
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "still here");
    }

    #[test]
    fn refresh_drops_files_that_are_gone_and_updates_sizes() {
        let scratch = Scratch::new("refresh");
        let gone = scratch.file("gone.txt", "x");
        let grown = scratch.file("grown.txt", "x");
        let mut shelf = Shelf::default();
        shelf.add([&gone, &grown]);

        std::fs::remove_file(&gone).unwrap();
        std::fs::write(&grown, "longer").unwrap();
        shelf.refresh();

        let items = shelf.items();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name, "grown.txt");
        assert_eq!(items[0].size, Some(6));
    }
}
