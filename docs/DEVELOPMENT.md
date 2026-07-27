# Development and export process

## Code map

| File | Responsibility |
| --- | --- |
| `src/lib.rs` | TRUCE plugin logic, process callback, pass-through audio |
| `src/params.rs` | Stable 145-parameter contract and defaults |
| `src/midi.rs` | Framework-light MIDI note-to-pad mapping |
| `src/editor.rs` | Native egui layout and parameter bindings |
| `src/main.rs` | Standalone application entry point |
| `truce.toml` | Plugin identity, category, MIDI wiring, format metadata |
| `Cargo.toml` | Format features and dependencies |

Keep audio-thread code allocation-free. GUI code belongs in `editor.rs`;
format-specific metadata belongs in `truce.toml`. The eventual Buffer Uppercut
DSP engine should be a framework-light module called only from `process`.

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
5. build every advertised format
6. run available validators
7. install and rescan in a DAW
8. verify automation names/IDs and state recall
9. verify MIDI on all accepted notes and channels
10. verify mono/stereo layouts and host tempo changes

Build products, installed bundles, and caches stay out of git.
