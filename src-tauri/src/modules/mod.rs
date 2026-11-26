//! The modules Shelf is built from, and which of them run.
//!
//! A module is one ability of the app: the shelf today, Sift next. Modules are built into the app. Each has its own
//! folder here and describes itself with a [`Module`], and [`ALL`] lists them.
//!
//! Plain Rust: nothing in this file knows about Tauri. Starting the modules that run is done in `lib.rs`.
//!
//! To add a module: make its folder with a `MODULE` in it, add that to [`ALL`], then start it and list its commands
//! in `lib.rs`.

use crate::settings::Settings;

pub mod shelf;

/// A system Shelf can run on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum System {
    MacOs,
    Windows,
    /// Any other system, such as Linux. Shelf is not made for these: no module lists it, so none runs there.
    Other,
}

impl System {
    /// The system this copy of Shelf was built for. `cfg!` is decided when the app is compiled, not while it runs.
    pub fn current() -> Self {
        if cfg!(target_os = "macos") {
            Self::MacOs
        } else if cfg!(target_os = "windows") {
            Self::Windows
        } else {
            Self::Other
        }
    }
}

/// What a module says about itself.
#[derive(Debug)]
pub struct Module {
    /// Its name in the settings file and in the action log. Once people use a module, its id must not change.
    pub id: &'static str,
    /// Its name for people.
    pub name: &'static str,
    /// The systems it works on.
    pub systems: &'static [System],
}

impl Module {
    pub fn supports(&self, system: System) -> bool {
        self.systems.contains(&system)
    }

    /// Whether the module runs: it must work on this system and be switched on in the settings. A module that does
    /// not run starts nothing.
    pub fn runs(&self, system: System, settings: &Settings) -> bool {
        self.supports(system) && settings.is_on(self.id)
    }
}

/// Every module, in the order the interface shows them.
pub const ALL: &[&Module] = &[&shelf::MODULE];

/// The settings of a new installation: every module is listed, and on. A module missing from the list would be on
/// too. Listing them all gives a file that shows what there is to switch off.
pub fn default_settings() -> Settings {
    let mut settings = Settings::default();
    for module in ALL {
        settings.modules.insert(module.id.to_owned(), true);
    }
    settings
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_two_modules_share_an_id() {
        for (index, module) in ALL.iter().enumerate() {
            assert!(
                ALL[..index].iter().all(|earlier| earlier.id != module.id),
                "two modules are called {}",
                module.id
            );
        }
    }

    #[test]
    fn a_module_supports_only_the_systems_it_lists() {
        let mac_only = Module {
            id: "mac-only",
            name: "Mac only",
            systems: &[System::MacOs],
        };

        assert!(mac_only.supports(System::MacOs));
        assert!(!mac_only.supports(System::Windows));
        assert!(!mac_only.supports(System::Other));
    }

    #[test]
    fn the_shelf_is_for_macos_and_windows() {
        assert!(shelf::MODULE.supports(System::MacOs));
        assert!(shelf::MODULE.supports(System::Windows));
        assert!(!shelf::MODULE.supports(System::Other));
    }

    #[test]
    fn with_the_default_settings_every_module_is_listed_and_on() {
        let settings = default_settings();

        for module in ALL {
            assert_eq!(settings.modules.get(module.id), Some(&true));
        }
        assert!(shelf::MODULE.runs(System::MacOs, &settings));
        assert!(shelf::MODULE.runs(System::Windows, &settings));
    }

    #[test]
    fn a_module_switched_off_does_not_run() {
        let mut settings = default_settings();
        settings.modules.insert(shelf::MODULE.id.to_owned(), false);

        assert!(!shelf::MODULE.runs(System::MacOs, &settings));
    }

    #[test]
    fn a_module_does_not_run_on_a_system_it_does_not_support() {
        // On in the settings, but that is not enough.
        let settings = default_settings();

        assert!(!shelf::MODULE.runs(System::Other, &settings));
    }
}
