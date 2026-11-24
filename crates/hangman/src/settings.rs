//! Saving the player's settings between sessions.
//!
//! A tiny plain-text file (`difficulty=hard`) in the platform's per-user config folder:
//!
//! | OS      | Location                                            |
//! |---------|-----------------------------------------------------|
//! | Linux   | `$XDG_CONFIG_HOME/hangman/settings.txt` (default `~/.config/...`) |
//! | macOS   | `~/Library/Application Support/hangman/settings.txt`|
//! | Windows | `%APPDATA%\hangman\settings.txt`                    |
//!
//! Every failure (no home dir, read-only disk, garbled file) is non-fatal: the game just
//! falls back to the default and carries on.

use std::fs;
use std::path::PathBuf;

use hangman_core::Difficulty;

const FILE_NAME: &str = "settings.txt";

#[cfg(target_os = "windows")]
fn config_root() -> Option<PathBuf> {
    std::env::var_os("APPDATA").map(PathBuf::from)
}

#[cfg(target_os = "macos")]
fn config_root() -> Option<PathBuf> {
    std::env::var_os("HOME").map(|h| PathBuf::from(h).join("Library/Application Support"))
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn config_root() -> Option<PathBuf> {
    // XDG: an unset, empty, or relative XDG_CONFIG_HOME must be ignored.
    if let Some(dir) = std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from) {
        if dir.is_absolute() {
            return Some(dir);
        }
    }
    std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config"))
}

fn settings_path() -> Option<PathBuf> {
    config_root().map(|root| root.join("hangman").join(FILE_NAME))
}

/// Text written to the settings file.
fn render(difficulty: Difficulty) -> String {
    format!("difficulty={}\n", difficulty.name())
}

/// Read the difficulty out of settings-file text. Unknown keys and bad values are ignored.
fn parse(text: &str) -> Option<Difficulty> {
    text.lines()
        .filter_map(|line| line.split_once('='))
        .find(|(key, _)| key.trim() == "difficulty")
        .and_then(|(_, value)| Difficulty::from_name(value))
}

/// The saved difficulty, if there is a readable settings file with a valid value.
pub fn load_difficulty() -> Option<Difficulty> {
    parse(&fs::read_to_string(settings_path()?).ok()?)
}

/// Save the difficulty. Failures are logged and otherwise ignored.
pub fn save_difficulty(difficulty: Difficulty) {
    let Some(path) = settings_path() else {
        return;
    };
    let result = (|| -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        // Write to a temp file then rename, so a crash can't leave a half-written file.
        let tmp = path.with_extension("tmp");
        fs::write(&tmp, render(difficulty))?;
        fs::rename(&tmp, &path)
    })();
    if let Err(err) = result {
        bevy::log::warn!("could not save settings to {}: {err}", path.display());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_then_parse_round_trips() {
        for d in [Difficulty::Easy, Difficulty::Hard] {
            assert_eq!(parse(&render(d)), Some(d));
        }
    }

    #[test]
    fn parse_tolerates_noise() {
        assert_eq!(
            parse("# comment\nvolume=3\n  difficulty = HARD  \n"),
            Some(Difficulty::Hard)
        );
    }

    #[test]
    fn parse_rejects_garbage() {
        assert_eq!(parse(""), None);
        assert_eq!(parse("difficulty=impossible"), None);
        assert_eq!(parse("difficulty"), None);
        assert_eq!(parse("hard"), None);
    }

    #[test]
    fn path_ends_with_app_folder_and_file() {
        if let Some(p) = settings_path() {
            assert!(p.ends_with(std::path::Path::new("hangman").join(FILE_NAME)));
        }
    }
}
