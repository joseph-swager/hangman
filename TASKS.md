# Hangman — task list

Goal: a finished Bevy hangman game that runs on this machine (Linux/Omarchy) and installs as an app, with a path to macOS + Windows builds.

- [x] 1. Workspace scaffold (core lib + bevy app)
- [x] 2. Core game logic (guess, win/lose, masked word, tests)
- [x] 3. Word list embedded in binary + random picker
- [x] 4. Bevy app: window, camera, gallows drawing (stages 0-6)
- [x] 5. Bevy app: masked word + wrong letters + status text
- [x] 6. Input: physical keyboard A-Z, on-screen letter buttons (mouse click)
- [x] 7. Game states: playing / won / lost, restart with Enter or button
- [x] 8. Build release, run and verify on this machine (screenshot)
- [x] 9. Linux install: .desktop file, icon, install script (~/.local)
- [x] 10. CI: GitHub Actions matrix (linux/macos/windows), release artifacts
- [x] 11. README (build, run, install, controls, platform notes)
- [x] 12. Final check: cargo test, clippy, fmt

## Round 2: difficulty levels (hints)
- [x] 13. Word data: every word gets a category + short clue (word|category|clue)
- [x] 14. Core: Entry parsing, Difficulty enum, Game carries category/clue, tests
- [x] 15. App: hint panel (Easy) / "no hints" (Hard), mode toggle (Tab + button), new game on toggle
- [x] 16. Rebuild, verify both modes visually, reinstall, update README

## Round 3: settings menu
- [x] 17. Gear icon button (top-right), drawn in code
- [x] 18. Settings overlay: choose Easy/Hard, shows current, Close button, Esc toggles
- [x] 19. Block game input while menu is open; remove old Mode button
- [x] 20. Build, verify (open/close, switch mode, input blocked), reinstall, README

## Round 4: remember difficulty between sessions
- [x] 21. Core: Difficulty name()/from_name() + tests
- [x] 22. App: settings module (config path per OS, parse/render, load/save) + tests
- [x] 23. Load at startup, save on change; HANGMAN_MODE env still overrides
- [x] 24. Verify persistence across real launches (isolated config dir), reinstall, README
