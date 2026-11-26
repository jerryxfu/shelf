//! Shelf's settings: one JSON file in the app's data folder, laid out to be read and edited by hand.
//!
//! ```json
//! {
//!   "modules": {
//!     "shelf": true
//!   }
//! }
//! ```
//!
//! Plain Rust: nothing here knows about Tauri. The caller says which folder the file is in.

use std::{collections::BTreeMap, fs, io, path::Path};

use serde::{Deserialize, Serialize};

pub const FILE: &str = "settings.json";

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Settings {
    /// Which modules are on, by id. A `BTreeMap` keeps its keys in order, so the file lists the modules the same way
    /// every time it is written. An id that no module has is kept as it is.
    #[serde(default)]
    pub modules: BTreeMap<String, bool>,
}

/// What reading the settings file gave.
#[derive(Debug, PartialEq)]
pub enum Loaded {
    Found(Settings),
    /// There is no file yet.
    Missing,
    /// There is a file, but it cannot be read or is not valid. The text says why. The file is left as it is: it may
    /// hold choices that only need a small fix by hand.
    Unreadable(String),
}

/// Reads the settings file in `dir`. It never changes anything on disk.
pub fn load(dir: &Path) -> Loaded {
    let text = match fs::read_to_string(dir.join(FILE)) {
        Ok(text) => text,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Loaded::Missing,
        Err(error) => return Loaded::Unreadable(error.to_string()),
    };
    // Some Windows editors put an invisible mark at the start of a file, which is not valid JSON.
    match serde_json::from_str(text.trim_start_matches('\u{feff}')) {
        Ok(settings) => Loaded::Found(settings),
        Err(error) => Loaded::Unreadable(error.to_string()),
    }
}

impl Settings {
    /// Whether a module is switched on. A module the file does not mention is on.
    pub fn is_on(&self, module: &str) -> bool {
        self.modules.get(module).copied().unwrap_or(true)
    }

    /// Writes the settings file in `dir`, in place of the one that was there. The folder is made if needed.
    pub fn save(&self, dir: &Path) -> io::Result<()> {
        let mut text = serde_json::to_string_pretty(self)?;
        text.push('\n');

        fs::create_dir_all(dir)?;
        // Written under another name first, then renamed: the settings file is always whole, even if Shelf stops
        // halfway through writing.
        let unfinished = dir.join(format!("{FILE}.tmp"));
        fs::write(&unfinished, text)?;
        fs::rename(unfinished, dir.join(FILE))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scratch::Scratch;

    #[test]
    fn with_no_file_nothing_is_found_and_every_module_is_on() {
        let scratch = Scratch::new("settings-missing");

        assert_eq!(load(scratch.dir()), Loaded::Missing);
        assert!(Settings::default().is_on("shelf"));
    }

    #[test]
    fn saved_settings_are_read_back() {
        let scratch = Scratch::new("settings-save");
        // A folder that is not there yet, as on the first start.
        let dir = scratch.dir().join("data");
        let mut settings = Settings::default();
        settings.modules.insert("shelf".into(), false);

        settings.save(&dir).unwrap();

        assert_eq!(load(&dir), Loaded::Found(settings.clone()));
        assert!(!settings.is_on("shelf"));
        // Laid out for a person: one module per line.
        let text = fs::read_to_string(dir.join(FILE)).unwrap();
        assert_eq!(text, "{\n  \"modules\": {\n    \"shelf\": false\n  }\n}\n");
        assert!(!dir.join(format!("{FILE}.tmp")).exists());
    }

    #[test]
    fn saving_again_replaces_the_file() {
        let scratch = Scratch::new("settings-replace");
        let mut settings = Settings::default();
        settings.modules.insert("shelf".into(), false);
        settings.save(scratch.dir()).unwrap();

        settings.modules.insert("shelf".into(), true);
        settings.save(scratch.dir()).unwrap();

        assert_eq!(load(scratch.dir()), Loaded::Found(settings));
    }

    #[test]
    fn a_file_that_is_not_valid_is_reported_and_left_as_it_is() {
        let scratch = Scratch::new("settings-invalid");
        // A slip of the hand, and a value of the wrong kind.
        for broken in [
            "{ \"modules\": { \"shelf\": fals } }",
            "{ \"modules\": { \"shelf\": \"no\" } }",
            "",
        ] {
            let file = scratch.file(FILE, broken);

            assert!(
                matches!(load(scratch.dir()), Loaded::Unreadable(_)),
                "read without a word: {broken:?}"
            );
            assert_eq!(fs::read_to_string(file).unwrap(), broken);
        }
    }

    #[test]
    fn a_file_that_cannot_be_opened_is_reported() {
        let scratch = Scratch::new("settings-unreadable");
        // A folder where the file should be: it is there, but it cannot be read as a file.
        fs::create_dir(scratch.dir().join(FILE)).unwrap();

        assert!(matches!(load(scratch.dir()), Loaded::Unreadable(_)));
        assert!(scratch.dir().join(FILE).is_dir());
    }

    #[test]
    fn a_module_the_file_does_not_mention_is_on_and_an_unknown_one_is_kept() {
        let scratch = Scratch::new("settings-partial");
        scratch.file(FILE, "{ \"modules\": { \"from-a-newer-shelf\": false } }");

        let Loaded::Found(settings) = load(scratch.dir()) else {
            panic!("the file is valid");
        };

        assert!(settings.is_on("shelf"));
        assert_eq!(settings.modules.get("from-a-newer-shelf"), Some(&false));
    }

    #[test]
    fn an_empty_object_and_a_windows_mark_at_the_start_are_fine() {
        let scratch = Scratch::new("settings-lenient");

        scratch.file(FILE, "{}");
        assert_eq!(load(scratch.dir()), Loaded::Found(Settings::default()));

        scratch.file(FILE, "\u{feff}{ \"modules\": { \"shelf\": false } }");
        let Loaded::Found(settings) = load(scratch.dir()) else {
            panic!("the mark at the start is skipped");
        };
        assert!(!settings.is_on("shelf"));
    }
}
