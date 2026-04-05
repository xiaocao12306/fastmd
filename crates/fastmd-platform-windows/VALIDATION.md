# `fastmd-platform-windows` Validation Notes

Reference surface:

- `apps/macos`

Stage 2 target locked by this crate:

- Windows 11
- Explorer

This validation file is crate-local evidence only. It does not claim full Windows parity and it does not replace the Stage 2 layer gates.

## Implemented in this slice

- buildable Rust library crate created
- Windows 11 + Explorer-only target encoded in crate docs and constants
- macOS reference behavior encoded as crate-local parity metadata
- crate-local local `.md` acceptance filtering implemented to mirror the macOS file checks
- unit tests added for local Markdown acceptance and rejection behavior

## Still pending

- frontmost Explorer detection
- Explorer hovered-item resolution
- coordinate translation and placement parity
- preview interaction parity wiring
- runtime diagnostics parity

## Verification commands

Run from the repository root:

```bash
cargo check --manifest-path crates/fastmd-platform-windows/Cargo.toml
```

Crate-local tests:

```bash
cargo test --manifest-path crates/fastmd-platform-windows/Cargo.toml
```

## Actual results in this worker clone

- `cargo check --manifest-path crates/fastmd-platform-windows/Cargo.toml`: passed
- `cargo check --tests --manifest-path crates/fastmd-platform-windows/Cargo.toml`: passed
- `cargo test --manifest-path crates/fastmd-platform-windows/Cargo.toml`: blocked by the current x86_64/Rosetta macOS linker environment failing to resolve `MacOSX.sdk` through `xcrun`
- `cargo test --target aarch64-apple-darwin --manifest-path crates/fastmd-platform-windows/Cargo.toml`: blocked because the `aarch64-apple-darwin` Rust target is not installed in this worker clone
