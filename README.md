# Keyboard Clicker

A cross-platform mouse and keyboard automation application. The repository keeps the original Python application and adds a separate Rust desktop application.

## Rust edition

The Rust application targets Windows 10/11 and Linux. It uses a native desktop interface and stores settings in the current user's configuration directory. The default UI language is English. German and Russian are included.

### Action sequences

Build a sequence from any combination of mouse clicks, named key presses, text entry, and waits. Actions run from top to bottom, can be enabled or disabled, reordered, and repeated as a complete cycle. Set a finite repeat count or use `0` to continue until stopped. F6 starts and F7 stops by default. Change both global hotkeys under Appearance; modifier combinations such as `Ctrl+F6` are supported.

The input worker is separate from the UI. Stop requests interrupt waits quickly, and held mouse or keyboard inputs are released before the worker exits.

### Appearance and languages

Choose a light or dark theme and set a custom accent color with a six-digit HEX value. To add a language, place a UTF-8 JSON file such as `pt-BR.json` in the `locales` directory inside the app's user configuration folder. Use `locales/en.json` as the key reference. Missing translation keys fall back to English. Restart the app after adding a translation.

User settings are stored in `settings.json` in that same per-user configuration directory. The settings format is JSON and does not require administrator permissions.

### Build from source

Install Rust stable and the native build dependencies for your Linux distribution. On Ubuntu/Debian, install:

```bash
sudo apt-get install libgtk-3-dev libxdo-dev libx11-dev libxcb-shape0-dev libxcb-xfixes0-dev pkg-config
```

Then build and test:

```bash
cargo fmt --all -- --check
cargo test --locked
cargo build --release --locked
```

The release binary is written to `target/release/keyboard-clicker` on Linux or `target/release/keyboard-clicker.exe` on Windows. GitHub Actions builds and tests both platforms. Push a semantic version tag such as `v2.0.0` to create a GitHub Release with platform archives.

Linux input automation and global hotkeys depend on the desktop session. X11 is the supported path for input simulation. Wayland-native behavior varies by compositor and may be restricted. The application still provides Start and Stop controls in its window.

## Original Python edition

The original Python application remains in the repository and can still be started independently:

```bash
python -m pip install -r requirements.txt
python main.py
```

## Inspiration and licensing

The Rust edition is an independent implementation. The project takes high-level product inspiration from [Blur Auto Clicker](https://github.com/Blur009/Blur-AutoClicker), but does not copy its source code or assets. The reference project is GPL-3.0; this repository's original project license remains in `LICENSE`.

## License

This project is licensed under the MIT License. See [LICENSE](LICENSE).
