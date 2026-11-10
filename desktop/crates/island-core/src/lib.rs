//! Platform-neutral logic of the island, shared by the desktop apps
//! (Windows now, macOS later). No OS dependencies, so everything here is
//! tested on any machine.
//!
//! It mirrors the pure JavaScript of the Omarchy plugin at the repo root
//! (`LyricsModel.js`, `IslandSettings.js`); both sides keep the same limits.

pub mod lyrics;
pub mod media;
pub mod monitor;
pub mod settings;
pub mod shape;
pub mod spring;
pub mod texts;

/// Version of the desktop apps, from the workspace.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    #[test]
    fn version_is_set() {
        assert!(!super::VERSION.is_empty());
    }
}
