# Buffer Uppercut

Buffer Uppercut is a real-time buffer performance audio effect built in Rust
with TRUCE 6.3 and a native Slint interface. This repository is the canonical
product implementation.

It provides:

- 16 momentary performance pads driven by the UI, automation, or MIDI notes
  60–75;
- beat repeat, reverse, tape stop, gate, pitch actions, low/mid/high bands,
  and LoFi effects;
- seven effect-aware macros per pad with semantic labels and values;
- a stereo history/captured-slice waveform;
- four factory kits, host-native preset recall, and a versioned `.bupreset`
  codec;
- host tempo, automation, state recall, MIDI illumination, and MIDI
  auto-selection;
- CLAP, VST3, and standalone targets;
- a framework-neutral planar `f64` DSP core with allocation-free processing
  after activation.

Earlier C++ and WRAC implementations were experiments. They are not supported
state, preset, product-ID, or source-compatibility targets.

## Quick start

Prerequisites:

- Rust 1.92 or newer
- `cargo-truce` 6.3
- platform audio/window development libraries
- Xcode command-line tools on macOS

```sh
rustup update stable
cargo install cargo-truce --version 6.3.0 --locked
cargo truce doctor
cargo truce run
```

Build and install the plugin formats:

```sh
cargo truce build --clap --vst3
cargo truce install --clap --vst3
```

Artifacts are written to `target/bundles/`. On macOS the installed bundles are:

- `~/Library/Audio/Plug-Ins/CLAP/Buffer Uppercut.clap`
- `~/Library/Audio/Plug-Ins/VST3/Buffer Uppercut.vst3`

Quit and reopen the DAW after installing because hosts commonly retain loaded
plugin binaries for the process lifetime.

## Verify

```sh
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
cargo test --locked --workspace
cargo test --locked --workspace --features rt-paranoid
cargo truce build --clap --vst3
```

Render or check the Slint editor:

```sh
cargo truce screenshot --out /tmp/buffer-uppercut.png --debug --scale 1
cargo truce screenshot --out screenshots/slint-default.png --check --debug --scale 1
BUFFER_UPPERCUT_EDITOR_SIZE=920x620 cargo truce screenshot --out screenshots/slint-minimum.png --check --debug --scale 1
BUFFER_UPPERCUT_EDITOR_SIZE=1440x760 cargo truce screenshot --out screenshots/slint-wide.png --check --debug --scale 1
BUFFER_UPPERCUT_EDITOR_PREVIEW=captured cargo truce screenshot --out screenshots/slint-captured.png --check --debug --scale 1
```

## Documentation

- [Architecture](docs/ARCHITECTURE.md): system boundaries and runtime flows
- [Contracts](docs/CONTRACTS.md): parameters, MIDI, realtime, kits, and state
- [Development](docs/DEVELOPMENT.md): common implementation workflows
- [Testing](docs/TESTING.md): automated and DAW acceptance procedures
- [Roadmap](docs/ROADMAP.md): current status, next work, and deferred scope
- [Decision records](docs/adr/README.md): architectural rationale

New agents must begin with [AGENTS.md](AGENTS.md).
