# Development

Read [Architecture](ARCHITECTURE.md) and [Contracts](CONTRACTS.md) before
changing behavior.

## Normal loop

```sh
cargo truce doctor
cargo test --locked --workspace
cargo truce run
```

The standalone app is the fastest way to inspect UI and DSP behavior. Use its
audio and MIDI menus to select devices.

Before handing off:

```sh
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
cargo test --locked --workspace
cargo test --locked --workspace --features rt-paranoid
cargo truce build --clap --vst3
```

See [Testing](TESTING.md) for validators, screenshots, benchmarks, and DAW
checks.

## Common changes

### Change DSP behavior

1. Work in `dsp/src/lib.rs`.
2. Keep `dsp` free of TRUCE, Slint, host, and filesystem types.
3. Add focused unit tests.
4. If expected audio changes intentionally, create a new versioned regression
   fixture set and record the decision in an ADR.
5. Run `cargo test -p buffer-uppercut-dsp`.
6. Run the workspace and `rt-paranoid` suites.

### Change a parameter

1. Update `src/params.rs`.
2. Update parameter-to-DSP translation in `src/lib.rs`.
3. Update editor bindings in `src/editor.rs` and Slint only if visible.
4. Update the schema in `CONTRACTS.md`.
5. Update schema, automation, state, and DSP tests.

The product is not bound to archived parameter IDs, but changing the canonical
schema must be deliberate and complete.

### Change MIDI behavior

1. Update the framework-light mapping in `src/midi.rs`.
2. Update aggregate held/pitch-transition logic in `src/lib.rs` if needed.
3. Test all channels, aliases, press/release ordering, and overlapping holds.
4. Update `CONTRACTS.md` and the REAPER checklist.

### Change or add an effect macro

Effect control names, enabled-state metadata, semantic formatting, defaults,
and DSP interpretation belong together in `dsp/src/lib.rs`. Update all of them
and verify the selected-pad Slint view.

### Change factory kits or the kit format

Factory definitions and serialization live in `kit/src/lib.rs`. File dialogs,
automation writes, and editor status belong in `src/editor.rs`.

A kit-format change increments `KIT_VERSION`, updates codec tests and
`CONTRACTS.md`, and records whether older native versions are intentionally
accepted or rejected. Never read or write files from `process`.

### Change the Slint editor

Visual components and layout live in `ui/main.slint`; host bindings and dynamic
path generation live in `src/editor.rs`.

```sh
cargo truce run
cargo truce screenshot --out /tmp/buffer-uppercut.png --debug --scale 1
```

Refresh baselines only after reviewing the rendered result:

```sh
cargo truce screenshot --out screenshots/slint-default.png --debug --scale 1
BUFFER_UPPERCUT_EDITOR_SIZE=920x620 cargo truce screenshot --out screenshots/slint-minimum.png --debug --scale 1
BUFFER_UPPERCUT_EDITOR_SIZE=1440x760 cargo truce screenshot --out screenshots/slint-wide.png --debug --scale 1
BUFFER_UPPERCUT_EDITOR_PREVIEW=captured cargo truce screenshot --out screenshots/slint-captured.png --debug --scale 1
```

Custom Slint editor changes require rebuilding and closing/reopening the plugin
editor even when using the TRUCE shell.

### Change a framework patch

Read the patch directory's `PATCH.md`. Keep patches narrow, version-locked, and
covered by focused tests. Record the upstream issue/release and exact removal
condition. Product features belong outside `vendor/`.

## Build and install

```sh
cargo truce build --clap
cargo truce build --vst3
cargo truce build --clap --vst3
cargo truce install --clap --vst3
```

The standalone release binary is:

```sh
cargo build --locked --release --bin buffer-uppercut-standalone
```

### Shell reload

```sh
cargo truce install --shell --vst3 --debug
```

Re-run after logic changes. Close and reopen custom editors after Slint or Rust
binding changes.

### Add another format

1. Add its optional wrapper dependency and feature in `Cargo.toml`.
2. Add required metadata to `truce.toml`.
3. Run `cargo truce doctor` and install the platform SDK.
4. Build, validate, and load it in a real host.
5. Add the format to CI and `TESTING.md`.

Do not advertise a format before CI and host validation both pass.

## Performance

```sh
cargo run --locked -p buffer-uppercut-dsp --release --example benchmark
```

Record the machine, OS, CPU, Rust version, sample rate, block size, and median
result. Compare like-for-like runs. Any repeatable audio-callback regression
above 10% requires investigation or an explicit acceptance decision.

## Documentation maintenance

- Stable rules and interfaces: `CONTRACTS.md`
- Structure and ownership: `ARCHITECTURE.md`
- Commands and implementation recipes: this file
- Acceptance procedures: `TESTING.md`
- Current status only: `ROADMAP.md`
- Architectural rationale: `adr/`

Update the authoritative file instead of copying the same information into
multiple documents.
