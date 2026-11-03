# Keyboard Clicker

[![Build](https://github.com/Mr1Err0r1/Keyboard-Clicker/actions/workflows/build.yml/badge.svg?branch=main)](https://github.com/Mr1Err0r1/Keyboard-Clicker/actions/workflows/build.yml) [![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

A cross-platform mouse and keyboard automation tool. The repository preserves the original Python application and provides a separate native Rust desktop application for Windows and Linux.

## Downloads

Get stable, versioned builds from [GitHub Releases](https://github.com/Mr1Err0r1/Keyboard-Clicker/releases). Windows downloads are ZIP files containing a portable x64 `.exe`; Linux downloads are x86_64 tarballs. Development builds are published as short-lived artifacts on the [Actions page](https://github.com/Mr1Err0r1/Keyboard-Clicker/actions) and are retained for 14 days.

The Windows executable is currently unsigned. Depending on Windows SmartScreen reputation and local policy, Windows may show a first-run warning. Use only releases from this repository and review the release checksums before running downloaded files.

## Rust application

The Rust app targets 64-bit Windows 10/11 and x86_64 Linux. The default interface language is English; German and Russian are included. The input worker runs separately from the UI and responds promptly to stop requests during waits.

### Build automation sequences

Build a connected, top-to-bottom flow of mouse clicks, mouse moves, key presses and combinations (for example `Ctrl+A`), text entry, and timed waits. Each numbered node can be enabled or disabled, edited, reordered, or removed. Set a delay after any action, then repeat a flow a chosen number of times or set the count to `0` to continue until stopped. Mouse clicks support left, right, and middle buttons, single/double clicks, held clicks, and optional fixed screen coordinates. Mouse-move nodes use explicit screen coordinates.

The editor saves settings automatically and can import/export validated JSON settings files for backup or sharing. Imported flows are limited to 256 steps and waits/holds of at most 24 hours each. Start and Stop remain available in the window, and global hotkeys work when supported by the desktop session. The automation runs in a separate worker; stop requests interrupt waits and held keys/buttons are released before the worker exits.

The default global hotkeys are F6 to start and F7 to stop. Change them in Appearance; single keys and modifier combinations such as `Ctrl+F6` are supported. The start and stop shortcuts must be different. Start and Stop buttons remain available in the window if the desktop session does not allow global shortcuts.

### Appearance, languages, and settings

Choose a dark or light theme and set a custom six-digit HEX accent color. To add a language, place a UTF-8 JSON file such as `pt-BR.json` in the `locales` folder under the app's configuration directory. Use [`locales/en.json`](locales/en.json) as the key reference. Missing translation keys fall back to English; restart the app after adding a language.

Settings are stored in `settings.json` under `KeyboardClicker` in the current user's configuration directory: normally `%APPDATA%\KeyboardClicker` on Windows and `$XDG_CONFIG_HOME/KeyboardClicker` (usually `~/.config/KeyboardClicker`) on Linux. No administrator privileges are required.

### Platform notes

On Windows, use the x64 executable on Windows 10 or 11. On Linux, input simulation currently uses the X11 input path; Wayland-native behavior varies by compositor and may be restricted. Global hotkeys can also depend on desktop-session permissions. The published Linux tarball contains the application binary, not a copy of your system libraries.

### Build from source

Install Rust stable. On Ubuntu/Debian, install the native build dependencies:

```bash
sudo apt-get update
sudo apt-get install libgtk-3-dev libxdo-dev libx11-dev libxcb-shape0-dev libxcb-xfixes0-dev pkg-config
```

Then run the quality checks and build:

```bash
cargo fmt --all -- --check
cargo test --locked
cargo clippy --all-targets --locked -- -D warnings
cargo build --release --locked
```

The binary is written to `target/release/keyboard-clicker` on Linux and `target/release/keyboard-clicker.exe` on Windows. GitHub Actions tests both systems. Pushing a semantic version tag such as `v2.0.0` builds x86_64 Windows and Linux release archives and attaches them to a GitHub Release. See [`CHANGELOG.md`](CHANGELOG.md) and the version-matched notes under [`.github/release-notes`](.github/release-notes).

## Repository layout

The active Rust application lives in `src/`; the preserved Python edition lives in `main.py` and `gui/`. Built-in translations are in `locales/`, production and concept artwork are under `assets/`, and GitHub automation/templates are under `.github/`. [`AGENTS.md`](AGENTS.md) is the concise architecture and workflow guide for AI coding agents.

## Original Python application

The existing Python files remain available and are not replaced by the Rust app. To run that edition:

```bash
python -m pip install -r requirements.txt
python main.py
```

## Contributing and security

Read [`CONTRIBUTING.md`](CONTRIBUTING.md) before opening a pull request. Please report security issues privately using the process in [`SECURITY.md`](SECURITY.md), rather than publishing exploit details in an issue.

## Inspiration and license

The Rust application is an independent implementation. It takes high-level product inspiration from [Blur Auto Clicker](https://github.com/Blur009/Blur-AutoClicker); it does not copy that project's source code or assets. This project is distributed under the MIT License; see [`LICENSE`](LICENSE).
