# AI Agent Guide — Keyboard Clicker

This file is the canonical repository guide for AI coding agents. Read it before editing. Keep it current when architecture, build commands, supported platforms, or release procedures change.

## Project at a glance

Keyboard Clicker is a desktop mouse/keyboard automation application. `main` contains the current Rust application (version `2.1.0`) and the preserved Python application. The original pre-Rust repository state is preserved on branch `old_original`; do not rewrite or delete that branch. The Rust app is the primary product direction. Python changes must be intentional and remain independent of Rust changes.

The default UI language is English, with German and Russian translations. The app targets x86_64 Windows 10/11 and x86_64 Linux. Linux input automation currently depends on X11; Wayland support is not guaranteed.

## Active Rust application map

| Path | Responsibility |
| --- | --- |
| `Cargo.toml`, `Cargo.lock` | Rust package, direct dependencies, locked dependency graph |
| `src/main.rs` | Native GUI process entry point and window startup |
| `src/lib.rs` | Exports active Rust modules |
| `src/app.rs` | Main egui user interface, settings interaction, hotkey registration |
| `src/model.rs` | Serializable action-sequence and settings data model |
| `src/engine.rs` | Background execution, input actions, cancellation and worker lifecycle |
| `src/config.rs` | Per-user settings path and persistence |
| `src/i18n.rs` | Built-in/external locale loading and English fallback |
| `src/theme.rs` | Theme and accent-color handling |
| `locales/en.json` | English source locale; add every new built-in UI key here |
| `locales/de.json`, `locales/ru.json` | Built-in German and Russian translations |

The old Rust prototypes were removed from active `src/` because the crate does not reference them. Historical source remains recoverable from Git history and `old_original`; do not reintroduce duplicate active module trees without a concrete use.

## Preserved Python application

`main.py`, `gui/`, `requirements.txt`, and `test.py` belong to the Python edition. Do not convert, rename, or remove them as part of Rust-only work. Python bytecode (`__pycache__`, `*.pyc`) is generated and must never be committed.

## Assets and concepts

`assets/` contains repository artwork. `assets/concepts/future-style/` and `assets/concepts/legacy-icons/` are preserved design exploration, not runtime dependencies. Keep new production assets distinct from concepts and document their license/source. Do not discard source art merely because current Rust code does not load it.

## Working rules

- Start from an up-to-date `main` and use a focused topic branch. Do not commit directly to `main` when the repo permits PR workflow.
- Preserve user work and the Python application. Avoid broad rewrites and unrelated formatting.
- Never commit secrets, credentials, personal settings, generated binaries, Cargo `target/`, Python caches, or local environment files.
- Keep the dependency lockfile in sync when changing Rust dependencies; favor small, justified updates. Major upgrades of `eframe`, `enigo`, or `global-hotkey` may require source/API migrations and both-platform testing. Do not merge a version bump solely because Dependabot opened it.
- Avoid unsafe input behavior. Automation must be user-configured, cancellable, and must release held keys/buttons when stopped or when an error occurs.
- Update the English, German, and Russian locale JSONs together for built-in UI text. Additional locales should fall back to English.
- Update `README.md`, `CHANGELOG.md`, and release notes when user-visible behavior or release contents change.

## Validation commands

Run from the repository root with Rust stable:

```bash
cargo fmt --all -- --check
cargo test --locked
cargo clippy --all-targets --locked -- -D warnings
cargo build --release --locked
```

On Ubuntu/Debian, native GUI/input build prerequisites are:

```bash
sudo apt-get install libgtk-3-dev libxdo-dev libx11-dev libxcb-shape0-dev libxcb-xfixes0-dev pkg-config
```

The GitHub Actions workflow `.github/workflows/build.yml` runs formatting, tests, and release builds on Windows and Linux. Validate behavior on both operating systems when platform-specific input code, hotkeys, GUI startup, or packaging changes.

## Releases and repository operations

`CHANGELOG.md` is the human-maintained change history. Each version has notes at `.github/release-notes/<tag>.md`. A semantic tag `vMAJOR.MINOR.PATCH` triggers the Windows x64 and Linux x86_64 packaging/publishing workflow. Release archives include `.sha256` sidecars. Verify the version, tag target, CI result, archive contents, and checksums before calling a release complete.

Use `CONTRIBUTING.md` for pull-request expectations, `SECURITY.md` for private vulnerability reports, and `.github/PULL_REQUEST_TEMPLATE.md` for contribution checks. Never force-push or delete user branches without explicit need and authorization. Do not merge major dependency PRs without reviewing their diffs, compatibility, and required CI.

## Source of truth

When documentation conflicts, prefer the actual source and CI configuration, then update the stale documentation. Keep this guide concise and accurate; do not copy the full README or changelog into it.
