# Contributing

Thanks for helping improve Keyboard Clicker. Contributions should target the Rust app unless a change explicitly concerns the preserved Python edition.

## Before opening a pull request

1. Open an issue first for major behavior changes or new dependencies.
2. Create a focused branch from `main`; keep changes scoped and describe user-visible changes.
3. Run `cargo fmt --all -- --check`, `cargo test --locked`, and `cargo clippy --all-targets --locked -- -D warnings`.
4. If changing a supported platform, verify the platform-specific build path and document any limitation.
5. Update `CHANGELOG.md`, translations, or README instructions when applicable.

The CI workflow builds and tests Windows and Linux. Tests should not require a real mouse, keyboard, display session, or administrator access. Avoid committing `target/`, local settings, secrets, or generated binaries.

## Translations

Keep the English locale as the key reference. Add each UI key to English, German, and Russian, and preserve JSON validity. Additional user locales may fall back to English for missing keys.

## Release process

Use semantic version tags in the form `vMAJOR.MINOR.PATCH`. Add version-specific notes at `.github/release-notes/<tag>.md` and update `CHANGELOG.md` before tagging. The workflow tests both platforms, packages Windows and Linux archives, and publishes the files to a GitHub Release.
