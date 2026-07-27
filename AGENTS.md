# AGENTS.md

## Project

Buffer Uppercut TRUCE Preview is the TRUCE 6.3 + native egui comparison
implementation. Its current product milestone matches the WRAC preview:
145 parameters, MIDI, tempo, state, pass-through DSP, CLAP/VST3/standalone.

## Working rules

- Keep the public parameter IDs and MIDI map compatible with the WRAC preview.
- Keep framework-light behavior in `params.rs`, `midi.rs`, and the future DSP
  engine. Keep TRUCE wrapper logic in `lib.rs` and egui code in `editor.rs`.
- Do not add WebView dependencies; this repository tests the native GUI path.
- Do not claim DSP parity while `process` is intentionally pass-through.
- Do not commit `target/`, installed bundles, or dependency caches.

## Verification

```sh
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo truce build --clap --vst3
```

Run the native editor with:

```sh
cargo truce run
```
