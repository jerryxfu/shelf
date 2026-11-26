//! The action log: every file Shelf moves or renames is written down here, so that it can be put back.
//!
//! It is in the core and shared by all modules. A module does not move a file itself: it asks the log to. That keeps
//! two promises in one place: nothing is ever deleted or overwritten, and every move can be undone.
//!
//! The log is one text file in the app's data folder, with one line of JSON per action, oldest first:
//!
//! ```json
//! {"id":1,"at":"2026-11-25T23:50:12Z","module":"sift","from":"/a/lab.pdf","to":"/b/lab.pdf","reason":"..."}
//! ```
//!
//! Lines are only ever added. An undo is a line of its own that names the action it took back.
//!
//! Plain Rust: nothing here knows about Tauri. The caller says which folder the file is in.

// No module moves files yet: Sift will be the first. Until then most of this file is only used by its tests, and
// this line tells the compiler that this is expected. When Sift calls it, the compiler will ask to remove the line.
#![cfg_attr(not(test), expect(dead_code, reason = "nothing moves files before Sift"))]

use std::{
    fmt, fs, io,
    io::Write,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

pub const FILE: &str = "actions.jsonl";

/// One line of the log: a file that was moved or renamed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Action {
    /// Counts up from 1.
    pub id: u64,
    /// When, in universal time (UTC): `2026-11-25T23:50:12Z`.
    pub at: String,
    /// The id of the module that asked.
    pub module: String,
    pub from: PathBuf,
    pub to: PathBuf,
    /// Why, in one line, for the person reading the log.
    pub reason: String,
    /// Only for an undo: the id of the action it took back. Left out of the line otherwise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub undoes: Option<u64>,
}

/// Why a move or an undo was refused. Whatever the reason, the file is where it was before.
#[derive(Debug)]
pub enum Error {
    /// The file to move is not there. For an undo: it is no longer where the action left it.
    Missing(PathBuf),
    /// Something is already at the place the file would go. Shelf never overwrites.
    InTheWay(PathBuf),
    /// The two places are on different disks. A plain rename cannot do that, and nothing else is tried.
    OtherDisk { from: PathBuf, to: PathBuf },
    /// The log has no action with this id.
    Unknown(u64),
    /// The action was undone before.
    AlreadyUndone(u64),
    /// The action is itself an undo.
    IsAnUndo(u64),
    /// The system said no, while reading or writing the log or while moving the file.
    Io(io::Error),
}

pub struct ActionLog {
    file: PathBuf,
}

impl ActionLog {
    /// The log kept in `dir`. Nothing is read or written before it is used.
    pub fn new(dir: &Path) -> Self {
        Self { file: dir.join(FILE) }
    }

    /// Every action written down so far, oldest first. A line that cannot be understood is skipped.
    pub fn actions(&self) -> io::Result<Vec<Action>> {
        Ok(parse(&self.read()?))
    }

    /// Moves or renames a file or a folder, and writes it down. `module` is the id of the module asking, and
    /// `reason` says why in one line.
    ///
    /// Refuses when something is already at `to`. The folder `to` is in must exist.
    ///
    /// `&mut self` means the caller must have the log to itself. Two moves at once would otherwise pick the same id.
    pub fn move_file(&mut self, module: &str, from: &Path, to: &Path, reason: &str) -> Result<Action, Error> {
        self.record(module, from, to, reason, None)
    }

    /// Puts a file back where an action took it from, and writes that down too.
    ///
    /// Refuses when the file is no longer where the action left it, when something has taken its old place, when the
    /// action was undone before, and when the action is itself an undo.
    pub fn undo(&mut self, id: u64) -> Result<Action, Error> {
        let actions = self.actions()?;
        let action = actions
            .iter()
            .find(|action| action.id == id)
            .ok_or(Error::Unknown(id))?;
        if action.undoes.is_some() {
            return Err(Error::IsAnUndo(id));
        }
        if actions.iter().any(|later| later.undoes == Some(id)) {
            return Err(Error::AlreadyUndone(id));
        }

        let reason = format!("Undo of action {id}");
        self.record(&action.module, &action.to, &action.from, &reason, Some(id))
    }

    fn record(
        &mut self,
        module: &str,
        from: &Path,
        to: &Path,
        reason: &str,
        undoes: Option<u64>,
    ) -> Result<Action, Error> {
        if !exists(from)? {
            return Err(Error::Missing(from.to_path_buf()));
        }
        // The system's own rename would replace what is at `to` without a word.
        if exists(to)? {
            return Err(Error::InTheWay(to.to_path_buf()));
        }

        // The line is made and the log is opened before anything moves. A log that cannot be written, or a path that
        // cannot be written down (one that is not valid Unicode), stops the move here.
        let text = self.read()?;
        let action = Action {
            id: next_id(&text),
            at: now(),
            module: module.to_owned(),
            from: from.to_path_buf(),
            to: to.to_path_buf(),
            reason: reason.to_owned(),
            undoes,
        };
        let mut line = serde_json::to_string(&action).map_err(io::Error::from)?;
        line.push('\n');
        // A last line with no end (a hand edit, a write cut short) would swallow the new one.
        if !text.is_empty() && !text.ends_with('\n') {
            line.insert(0, '\n');
        }
        if let Some(dir) = self.file.parent() {
            fs::create_dir_all(dir)?;
        }
        let mut log = fs::OpenOptions::new().create(true).append(true).open(&self.file)?;

        fs::rename(from, to).map_err(|error| match error.kind() {
            io::ErrorKind::CrossesDevices => Error::OtherDisk {
                from: from.to_path_buf(),
                to: to.to_path_buf(),
            },
            _ => Error::Io(error),
        })?;

        if let Err(error) = log.write_all(line.as_bytes()) {
            // A move that is not written down could never be undone, so the file goes back.
            let _ = fs::rename(to, from);
            return Err(error.into());
        }
        Ok(action)
    }

    /// The log's text. No file yet is an empty log.
    fn read(&self) -> io::Result<String> {
        match fs::read_to_string(&self.file) {
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(String::new()),
            other => other,
        }
    }
}

fn parse(text: &str) -> Vec<Action> {
    text.lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect()
}

fn next_id(text: &str) -> u64 {
    parse(text)
        .iter()
        .map(|action| action.id)
        .max()
        .map_or(1, |last| last + 1)
}

/// Whether something is at `path`. Unlike `Path::exists`, a link that points nowhere counts: it is in the way too.
fn exists(path: &Path) -> io::Result<bool> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error),
    }
}

/// The time now, to the second, in the form `2026-11-25T23:50:12Z`.
fn now() -> String {
    let now = OffsetDateTime::now_utc();
    let now = now.replace_nanosecond(0).unwrap_or(now);
    now.format(&Rfc3339).unwrap_or_default()
}

/// How an error reads as text.
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Missing(path) => write!(f, "{} is not there", path.display()),
            Self::InTheWay(path) => write!(f, "{} already exists, and Shelf never overwrites", path.display()),
            Self::OtherDisk { from, to } => write!(
                f,
                "{} and {} are on different disks, and Shelf cannot move a file between disks yet",
                from.display(),
                to.display()
            ),
            Self::Unknown(id) => write!(f, "the log has no action {id}"),
            Self::AlreadyUndone(id) => write!(f, "action {id} was already undone"),
            Self::IsAnUndo(id) => write!(f, "action {id} is an undo, and an undo cannot be undone"),
            Self::Io(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for Error {}

/// Lets `?` turn a system error into ours.
impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scratch::Scratch;

    /// A log in a folder that is not there yet, as on the first start.
    fn log_in(scratch: &Scratch) -> ActionLog {
        ActionLog::new(&scratch.dir().join("data"))
    }

    fn lines(log: &ActionLog) -> usize {
        log.read().unwrap().lines().count()
    }

    #[test]
    fn a_move_is_done_and_written_down() {
        let scratch = Scratch::new("log-move");
        let from = scratch.file("lab.pdf", "the lab");
        let to = scratch.dir().join("lab 3.pdf");
        let mut log = log_in(&scratch);

        let action = log
            .move_file("sift", &from, &to, "The name matches the course")
            .unwrap();

        assert!(!from.exists());
        assert_eq!(fs::read_to_string(&to).unwrap(), "the lab");
        assert_eq!(action.id, 1);
        assert_eq!(action.module, "sift");
        assert_eq!(action.from, from);
        assert_eq!(action.to, to);
        assert_eq!(action.reason, "The name matches the course");
        assert_eq!(action.undoes, None);
        // `2026-11-25T23:50:12Z`: twenty characters, whole seconds, universal time.
        assert_eq!(action.at.len(), 20, "{}", action.at);
        assert!(action.at.ends_with('Z'));
        // What is read back from the file is what was done.
        assert_eq!(log.actions().unwrap(), [action]);
        assert_eq!(lines(&log), 1);
    }

    #[test]
    fn a_folder_can_be_moved_too_and_ids_count_up() {
        let scratch = Scratch::new("log-folder");
        let file = scratch.file("a.txt", "a");
        let folder = scratch.dir().join("notes");
        fs::create_dir(&folder).unwrap();
        let mut log = log_in(&scratch);

        let first = log.move_file("sift", &file, &folder.join("a.txt"), "first").unwrap();
        let second = log
            .move_file("sift", &folder, &scratch.dir().join("old notes"), "second")
            .unwrap();

        assert_eq!((first.id, second.id), (1, 2));
        assert_eq!(fs::read_to_string(scratch.dir().join("old notes/a.txt")).unwrap(), "a");
    }

    #[test]
    fn undo_moves_the_file_back_and_is_written_down() {
        let scratch = Scratch::new("log-undo");
        let from = scratch.file("a.txt", "a");
        let to = scratch.dir().join("b.txt");
        let mut log = log_in(&scratch);
        log.move_file("sift", &from, &to, "why").unwrap();

        let undo = log.undo(1).unwrap();

        assert_eq!(fs::read_to_string(&from).unwrap(), "a");
        assert!(!to.exists());
        assert_eq!(undo.id, 2);
        assert_eq!(undo.undoes, Some(1));
        assert_eq!(undo.module, "sift");
        assert_eq!((&undo.from, &undo.to), (&to, &from));
        assert_eq!(log.actions().unwrap().len(), 2);
    }

    #[test]
    fn undo_refuses_when_something_is_in_the_way() {
        let scratch = Scratch::new("log-undo-in-the-way");
        let from = scratch.file("a.txt", "moved");
        let to = scratch.dir().join("b.txt");
        let mut log = log_in(&scratch);
        log.move_file("sift", &from, &to, "why").unwrap();
        // Another file takes the old place.
        scratch.file("a.txt", "newer");

        let refused = log.undo(1);

        assert!(matches!(refused, Err(Error::InTheWay(path)) if path == from));
        assert_eq!(fs::read_to_string(&from).unwrap(), "newer");
        assert_eq!(fs::read_to_string(&to).unwrap(), "moved");
        assert_eq!(lines(&log), 1);
    }

    #[test]
    fn undo_refuses_when_the_file_is_no_longer_where_the_action_left_it() {
        let scratch = Scratch::new("log-undo-gone");
        let from = scratch.file("a.txt", "a");
        let to = scratch.dir().join("b.txt");
        let mut log = log_in(&scratch);
        log.move_file("sift", &from, &to, "why").unwrap();
        // Someone moves it on by hand.
        fs::rename(&to, scratch.dir().join("c.txt")).unwrap();

        let refused = log.undo(1);

        assert!(matches!(refused, Err(Error::Missing(path)) if path == to));
        assert!(!from.exists());
        assert_eq!(lines(&log), 1);
    }

    #[test]
    fn a_move_never_overwrites() {
        let scratch = Scratch::new("log-no-overwrite");
        let from = scratch.file("a.txt", "a");
        let to = scratch.file("b.txt", "b");
        let mut log = log_in(&scratch);

        let refused = log.move_file("sift", &from, &to, "why");

        assert!(matches!(refused, Err(Error::InTheWay(path)) if path == to));
        assert_eq!(fs::read_to_string(&from).unwrap(), "a");
        assert_eq!(fs::read_to_string(&to).unwrap(), "b");
        assert_eq!(lines(&log), 0);
    }

    #[test]
    fn a_file_that_is_not_there_is_not_moved() {
        let scratch = Scratch::new("log-missing");
        let from = scratch.dir().join("nothing.txt");
        let mut log = log_in(&scratch);

        let refused = log.move_file("sift", &from, &scratch.dir().join("b.txt"), "why");

        assert!(matches!(refused, Err(Error::Missing(path)) if path == from));
        assert_eq!(lines(&log), 0);
    }

    #[test]
    fn a_move_into_a_folder_that_does_not_exist_is_refused() {
        let scratch = Scratch::new("log-no-folder");
        let from = scratch.file("a.txt", "a");
        let mut log = log_in(&scratch);

        let refused = log.move_file("sift", &from, &scratch.dir().join("nowhere/a.txt"), "why");

        assert!(matches!(refused, Err(Error::Io(_))));
        assert_eq!(fs::read_to_string(&from).unwrap(), "a");
        assert_eq!(lines(&log), 0);
    }

    /// Needs a second disk, so it only runs when asked for, with a folder on that disk:
    /// `SHELF_TEST_OTHER_DISK=/Volumes/Other cargo test -- --ignored another_disk`
    #[test]
    #[ignore = "needs SHELF_TEST_OTHER_DISK, a folder on another disk"]
    fn a_move_to_another_disk_is_refused_and_the_file_stays() {
        let other = PathBuf::from(std::env::var_os("SHELF_TEST_OTHER_DISK").expect("SHELF_TEST_OTHER_DISK is not set"));
        let scratch = Scratch::new("log-other-disk");
        let from = scratch.file("a.txt", "a");
        let to = other.join(format!("shelf-test-{}.txt", std::process::id()));
        let mut log = log_in(&scratch);

        let refused = log.move_file("sift", &from, &to, "why");

        assert!(matches!(refused, Err(Error::OtherDisk { .. })), "{refused:?}");
        assert_eq!(fs::read_to_string(&from).unwrap(), "a");
        assert!(!to.exists());
        assert_eq!(lines(&log), 0);
    }

    #[test]
    fn an_action_is_undone_only_once_and_an_undo_is_not_undone() {
        let scratch = Scratch::new("log-undo-once");
        let from = scratch.file("a.txt", "a");
        let to = scratch.dir().join("b.txt");
        let mut log = log_in(&scratch);
        log.move_file("sift", &from, &to, "why").unwrap();
        log.undo(1).unwrap();
        // Back by hand to where the first action left it: a second undo would find nothing wrong on disk.
        fs::rename(&from, &to).unwrap();

        assert!(matches!(log.undo(1), Err(Error::AlreadyUndone(1))));
        assert!(matches!(log.undo(2), Err(Error::IsAnUndo(2))));
        assert!(matches!(log.undo(7), Err(Error::Unknown(7))));
        assert!(to.exists());
        assert_eq!(lines(&log), 2);
    }

    #[test]
    fn a_broken_line_is_skipped_and_does_not_swallow_the_next_one() {
        let scratch = Scratch::new("log-broken-line");
        let from = scratch.file("a.txt", "a");
        let to = scratch.dir().join("b.txt");
        let mut log = log_in(&scratch);
        log.move_file("sift", &from, &to, "why").unwrap();
        // A line cut short, with no end.
        let mut text = log.read().unwrap();
        text.push_str("{\"id\":2,\"at\":\"2026-11-2");
        fs::write(&log.file, text).unwrap();

        let undo = log.undo(1).unwrap();

        assert_eq!(undo.id, 2);
        let actions = log.actions().unwrap();
        assert_eq!(actions.len(), 2);
        assert_eq!(actions[1], undo);
        // The broken line is still in the file: the log only ever adds.
        assert_eq!(lines(&log), 3);
    }

    #[test]
    fn a_reason_on_two_lines_still_takes_one_line_of_the_log() {
        let scratch = Scratch::new("log-one-line");
        let from = scratch.file("été.txt", "a");
        let to = scratch.dir().join("hiver.txt");
        let mut log = log_in(&scratch);

        let action = log.move_file("sift", &from, &to, "first\nsecond").unwrap();

        assert_eq!(lines(&log), 1);
        assert_eq!(log.actions().unwrap(), [action]);
    }

    #[test]
    fn errors_read_as_plain_sentences() {
        let error = Error::InTheWay(PathBuf::from("/b/lab.pdf"));

        assert_eq!(
            error.to_string(),
            "/b/lab.pdf already exists, and Shelf never overwrites"
        );
    }
}
