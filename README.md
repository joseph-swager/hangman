# Hangman

A hangman game written in Rust with the [Bevy](https://bevyengine.org) engine (0.20).
Targets Linux, macOS and Windows.

## Play

- Type letters on your keyboard, or click the on-screen keys.
- 6 wrong guesses and the figure is complete.
- **Enter** (or the *New game* button) starts a new game.
- **Gear button** (top-right) or **Esc** opens the settings menu, where you choose the difficulty:
  - **Easy** (default): shows the word's category and a short clue.
  - **Hard**: no hints at all.
  
  Changing the difficulty starts a new game. While the menu is open the game ignores the keyboard.
- **Tab** flips Easy/Hard without opening the menu.
- **Quit**: the last button in the settings menu, or **Ctrl+Q** (**Cmd+Q** on macOS). On Windows and most Linux desktops **Alt+F4** and the title-bar close/minimize buttons also work.
- Your difficulty is remembered between sessions (see *Settings file* below).
- The window scales to any size, including tiled windows.

## Layout

| Crate | What it is |
|---|---|
| `crates/hangman-core` | Game rules and word list. No engine code; unit-tested. |
| `crates/hangman` | The Bevy app: rendering, input, UI. |

The word list (`crates/hangman-core/words.txt`) is embedded in the binary. One word per line as
`word|category|clue`; a unit test checks that every entry has a category, a clue of 5-40
characters, and a clue that doesn't contain the word itself.

## Build and run

```sh
cargo run -p hangman                 # dev build (first compile of Bevy takes a few minutes)
cargo run --release -p hangman       # optimized
cargo test --workspace
HANGMAN_WORD=rust cargo run -p hangman   # force the first word (for testing)
HANGMAN_MODE=hard cargo run -p hangman   # start in hard mode (overrides the saved setting)
```

### Linux build dependencies

Arch/Omarchy usually has everything already. On Debian/Ubuntu:

```sh
sudo apt install libasound2-dev libudev-dev libwayland-dev libxkbcommon-dev \
  libx11-dev libxcursor-dev libxi-dev libxrandr-dev
```

## Settings file

The chosen difficulty is saved to a one-line text file, `difficulty=easy` or `difficulty=hard`:

| OS | Location |
|---|---|
| Linux | `$XDG_CONFIG_HOME/hangman/settings.txt` (default `~/.config/hangman/settings.txt`) |
| macOS | `~/Library/Application Support/hangman/settings.txt` |
| Windows | `%APPDATA%\hangman\settings.txt` |

Delete the file to reset to Easy. If it can't be read or written, the game just uses the default.
`HANGMAN_MODE=easy|hard` overrides the saved value for one run.

## Install as an app (Linux, per-user, no sudo)

```sh
./install.sh              # builds release, installs to ~/.local, adds app-launcher entry
./install.sh --uninstall
```

Installs `~/.local/bin/hangman`, an icon, and `~/.local/share/applications/hangman.desktop`.

## macOS and Windows

The code is cross-platform, but you can't easily cross-compile from Linux.
`.github/workflows/ci.yml` builds, lints and tests on Linux, macOS and Windows, uploads the
binaries as artifacts, and attaches them to a GitHub release when you push a `v*` tag.
macOS binaries are unsigned; Gatekeeper will ask you to allow them on first run.

## Development notes

- Bevy's API changes between releases; the version is pinned in `crates/hangman/Cargo.toml`.
- Dev profile optimizes dependencies (`opt-level = 3`) because unoptimized Bevy is slow.
