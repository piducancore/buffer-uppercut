# Buffer Uppercut — TRUCE + native GUI comparison

This repository is the TRUCE 6.3 + egui comparison build of Buffer Uppercut.
It exists beside the WRAC/WebView preview so the two framework paths can be
evaluated at the same milestone.

## Current milestone

- 145 automatable parameters with the same IDs, names, ranges, and classic
  defaults as the WRAC preview
- MIDI notes 60–75 mapped to the 16 performance pads
- octave aliases for pitch notes 69–71
- MIDI pad illumination and optional pad auto-selection
- effect-aware macro labels and semantic values, with unused controls disabled
- host tempo/transport access
- native egui editor
- host-managed parameter state
- CLAP, VST3, and standalone targets
- independent, framework-free `f64` DSP core
- beat repeat, reverse, tape stop, gate, pitch actions, three bands, and LoFi
- pinned C++ behavior fixtures with `1e-7` absolute/relative tolerance
- allocation-free processing after activation/reset

The plugin has unique preview metadata, so it can be installed beside the
WRAC preview without replacing it.

## Prerequisites

- Rust 1.92 or newer
- `cargo-truce` 6.3
- Xcode command-line tools on macOS

```sh
rustup update stable
cargo install cargo-truce --version 6.3.0 --locked
cargo truce doctor
```

## Run the native app

```sh
cargo truce run
```

The standalone app is the quickest way to inspect and develop the native GUI.
Use its audio and MIDI menus to select devices.

## Build plugin bundles

```sh
cargo truce build --clap --vst3
```

Artifacts are written below `target/bundles/`.

## Install for a DAW

```sh
cargo truce install --clap --vst3
```

On macOS the user install locations are:

- `~/Library/Audio/Plug-Ins/CLAP`
- `~/Library/Audio/Plug-Ins/VST3`

Quit and reopen the DAW after installing. Hosts commonly retain loaded plugin
binaries for their whole process lifetime. Look for
**Buffer Uppercut TRUCE Preview**.

## Develop

```sh
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo test --features rt-paranoid
cargo truce build --clap --vst3
```

Use `cargo truce run` for normal editor iteration. The optional TRUCE shell
workflow can shorten plugin reload cycles:

```sh
cargo truce install --shell --vst3 --debug
```

Re-run that command after code changes to replace the shell-loaded logic
library while keeping the stable wrapper installed.

See [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md) for the code map, state and
real-time rules, validation flow, and how to add another export format.
See [docs/COMPARISON.md](docs/COMPARISON.md) for the controlled WRAC/TRUCE
comparison.

## DSP contract

The local `buffer-uppercut-dsp` crate is an independent Rust port. Its
integration tests consume the vendored `contract/` snapshot from C++ commit
`bc17659aa517b9910761c1861cadd873402b75de`. Verify the snapshot and render
fixtures with:

```sh
(cd contract && shasum -a 256 -c SHA256SUMS)
cargo test -p buffer-uppercut-dsp --test contract
```

See [contract/PINNED.md](contract/PINNED.md) before updating fixtures.
Parameter-state data remains native to each framework; cross-framework preset
migration and kit import/export are not part of this milestone.
