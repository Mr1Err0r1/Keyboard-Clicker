# Changelog

This project follows Semantic Versioning. Each published release should include tested Windows and Linux artifacts.

## [Unreleased]

### Changed
- Updated GitHub Actions and compatible Rust dependencies after cross-platform CI verification.

## [2.0.0] - 2026-11-03

### Added
- Independent Rust desktop application alongside the preserved Python app.
- Ordered action sequences that combine mouse clicks, key presses, text entry, and waits.
- Repeat-until-stopped and finite sequence cycles, reorderable steps, and per-action delays.
- English default UI with German and Russian translations and JSON-based language extensions.
- Dark/light themes and a user-selected accent color.
- User-configurable global start/stop hotkeys, including modifier combinations.
- Per-user JSON settings and Linux/Windows CI/release packaging.

### Fixed
- Sequence waits can be stopped without waiting for the full configured delay.
- Held mouse buttons and keyboard keys are released when a stop request arrives.
- Mouse and keyboard actions execute serially in one sequence worker rather than racing in separate threads.

[2.0.0]: https://github.com/Mr1Err0r1/Keyboard-Clicker/releases/tag/v2.0.0
