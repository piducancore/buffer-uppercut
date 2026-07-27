# AGENTS.md

## Project

Buffer Uppercut TRUCE Preview is the TRUCE 6.3 + native egui comparison
implementation. It has 145 parameters, MIDI, tempo, state, an independent
framework-free f64 DSP port, and CLAP/VST3/standalone exports.

## Working rules

- Keep the public parameter IDs and MIDI map compatible with the WRAC preview.
- Keep framework-light behavior in `params.rs`, `midi.rs`, and `dsp`. Keep
  TRUCE wrapper logic in `lib.rs` and egui code in `editor.rs`.
- Keep `dsp` independent of TRUCE and the WRAC Rust implementation. Its only
  behavior oracle is the pinned C++ contract in `contract`.
- Do not add WebView dependencies; this repository tests the native GUI path.
- Do not claim DSP parity while `process` is intentionally pass-through.
- Do not commit `target/`, installed bundles, or dependency caches.

## Verification

```sh
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo test --features rt-paranoid
cargo truce build --clap --vst3
```

Run the native editor with:

```sh
cargo truce run
```
