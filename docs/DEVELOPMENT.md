# Development and export process

## Code map

| File | Responsibility |
| --- | --- |
| `src/lib.rs` | TRUCE `f64` wrapper, parameter/MIDI aggregation, preallocated I/O |
| `src/params.rs` | Stable 145-parameter contract and defaults |
| `src/midi.rs` | Framework-light MIDI note-to-pad mapping |
| `src/editor.rs` | Native egui layout and parameter bindings |
| `src/main.rs` | Standalone application entry point |
| `dsp/src/lib.rs` | Independent, framework-free Buffer Uppercut DSP core |
| `contract/` | Pinned C++ fixture corpus and checksums |
| `truce.toml` | Plugin identity, category, MIDI wiring, format metadata |
| `Cargo.toml` | Format features and dependencies |
| `vendor/truce-clap/` | Exact TRUCE 6.3.0 CLAP wrapper plus documented state-rescan backport |

Keep audio-thread code allocation-free. `reset` owns ring-buffer and block
scratch allocation. GUI code belongs in `editor.rs`; format-specific metadata
belongs in `truce.toml`. The DSP crate must not depend on TRUCE or either Rust
comparison implementation.

TRUCE uses `PluginLogic64`. Format wrappers widen/narrow host buffers at their
boundary, while this plugin and its DSP fixtures remain planar `f64`.

The local `truce-clap` patch makes state and preset loads notify the CLAP host
that parameter values changed. Keep the patch version-locked, and remove it
when an upstream TRUCE release includes equivalent behavior. The patch's
provenance and removal condition are recorded in `vendor/truce-clap/PATCH.md`.

## Parameter compatibility

The public contract is:

- ID 0: performance pitch
- IDs 1–144: 16 pads × 9 parameters
- each pad: trigger, effect type, seven controls

Do not reorder or renumber parameters after release. Add tests whenever the
schema changes. TRUCE persists the parameter store through the host wrapper.
The WRAC and TRUCE byte-level state formats are not interchangeable yet, so a
future migration layer must translate by stable parameter ID.

## MIDI compatibility

Use MIDI note numbers in product-facing text because host octave labels vary.

| Note | Action |
| --- | --- |
| 60–75 | Pads 1–16 |
| 57–59 | octave-below aliases for pitch pads 10–12 |
| 81–83 | octave-above aliases for pitch pads 10–12 |

MIDI state is held independently for all 16 channels.

In REAPER, open **View → Virtual MIDI Keyboard**, route the track input from
the virtual keyboard, arm/monitor the track, and enable **Send all keyboard
input** when typing while another window is active. The local `truce-egui`
backport returns keyboard events to the parent DAW whenever no text editor is
focused, allowing REAPER's computer keyboard to continue generating MIDI while
the plugin editor is open. See `vendor/truce-egui/PATCH.md`.

Pitch pads update the internal performance pitch at the aggregate
released-to-held transition and emit a host parameter-change event. Audio code
must not mutate the host-owned parameter store directly; this preserves CLAP
process/flush equivalence and lets every wrapper record the change correctly.
Wrapper outputs are also normalized by flushing samples below the `f32` normal
range to zero before they cross either format boundary.

## Normal development loop

```sh
cargo truce doctor
cargo test
cargo truce run
```

Before committing:

```sh
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo truce build --clap --vst3
```

For allocation checks:

```sh
cargo test --features rt-paranoid
```

`cargo test` is workspace-wide: it runs wrapper tests, DSP unit tests, all 19
processing fixtures, and the pitch-action fixture. `rt-paranoid` enables
TRUCE's audio-thread allocation detector. Any allocation reported from
`process` is a blocking failure.

## DSP contract workflow

The canonical snapshot is `contract/dsp-contract-v1`, represented locally by
the files in `contract/`. It is pinned to C++ commit
`bc17659aa517b9910761c1861cadd873402b75de`; the manifest SHA-256 is recorded in
`contract/PINNED.md`.

```sh
(cd contract && shasum -a 256 -c SHA256SUMS)
cargo test -p buffer-uppercut-dsp --test contract
```

To sync a new contract, first create and review a new version in the C++ repo.
Copy its complete directory, update `contract/PINNED.md`, and run the entire
workspace suite. Never edit expected samples locally. Intentional behavior
changes need a versioned contract and deviation-ledger entry.

## Performance

Run the deterministic release benchmark:

```sh
cargo run -p buffer-uppercut-dsp --release --example benchmark
```

Record the reported nanoseconds per stereo frame with the machine, OS, CPU,
Rust version, and C++ reference result from the same machine. Compare medians
from at least five runs. A Rust median more than 25% slower than the pinned C++
engine blocks milestone sign-off unless explicitly accepted.

## Build and install variants

TRUCE format flags select independent wrappers:

```sh
cargo truce build --clap
cargo truce build --vst3
cargo truce build --clap --vst3
```

Replace `build` with `install` to copy bundles to the user plugin folders.
The standalone target is run with `cargo truce run`.

### Adding another TRUCE format

1. Add the optional wrapper dependency in `Cargo.toml`.
2. Add a feature that enables the dependency.
3. Add any required format metadata to `truce.toml`.
4. Run `cargo truce doctor` and install the platform SDK/validator it reports.
5. Build with the corresponding flag.
6. Validate in a format validator and at least one real host.
7. Document the artifact and install paths here.

Do not claim a format as supported until CI builds it and a host has loaded it.

## Shell reload workflow

TRUCE can install a stable shell and rebuild plugin logic separately:

```sh
cargo truce install --shell --vst3 --debug
```

Use the standalone app for GUI layout work unless the behavior specifically
depends on a DAW. Use shell reload for host integration and DSP iteration.
Re-run the shell install command after changes to rebuild the logic library.

## Release validation checklist

1. `cargo fmt --check`
2. `cargo clippy --all-targets --all-features -- -D warnings`
3. `cargo test`
4. `cargo test --features rt-paranoid`
5. `cargo run -p buffer-uppercut-dsp --release --example benchmark`
6. build every advertised format
7. run available CLAP/VST3 validators
8. install and rescan in a DAW
9. verify automation names/IDs and state recall
10. verify MIDI on all accepted notes and channels
11. verify every effect, mono/stereo layouts, and host tempo changes

Build products, installed bundles, and caches stay out of git.
