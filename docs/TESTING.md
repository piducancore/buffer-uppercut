# Testing and acceptance

## Test levels

### Fast local check

```sh
cargo test --locked --workspace
```

This runs DSP units, wrapper/MIDI/parameter/editor tests, native kit codec
tests, the frozen audio regression corpus, and framework-patch tests.

### Required before handoff

```sh
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
cargo test --locked --workspace
cargo test --locked --workspace --features rt-paranoid
cargo truce build --clap --vst3
```

`rt-paranoid` is blocking: any process-thread allocation fails acceptance.

### Regression-corpus integrity

```sh
(cd contract && shasum -a 256 -c SHA256SUMS)
cargo test -p buffer-uppercut-dsp --test contract
```

The corpus preserves the current sound. Its historical provenance is not a
backwards-compatibility requirement.

## Slint screenshots

Check all committed sizes:

```sh
cargo truce screenshot --out screenshots/slint-default.png --check --debug --scale 1
BUFFER_UPPERCUT_EDITOR_SIZE=920x620 cargo truce screenshot --out screenshots/slint-minimum.png --check --debug --scale 1
BUFFER_UPPERCUT_EDITOR_SIZE=1440x760 cargo truce screenshot --out screenshots/slint-wide.png --check --debug --scale 1
BUFFER_UPPERCUT_EDITOR_PREVIEW=captured cargo truce screenshot --out screenshots/slint-captured.png --check --debug --scale 1
```

The default, minimum, and wide baselines exercise rolling history. The captured
baseline exercises fixed layout geometry, capture emphasis, and held-pad
hierarchy.
Additional state baselines for MIDI-held, reverse, and disabled-macro states
remain roadmap work.

## Validators

Build release bundles first:

```sh
cargo truce build --clap --vst3
clap-validator validate "target/bundles/Buffer Uppercut.clap"
pluginval --strictness-level 10 --skip-gui-tests --validate "target/bundles/Buffer Uppercut.vst3"
```

Run pluginval GUI tests in an interactive desktop session when preparing a
release. Headless CI may use `--skip-gui-tests`.

## REAPER smoke test

Quit and reopen REAPER after installing a new build.

### Discovery and lifecycle

- VST3 and CLAP appear as **Buffer Uppercut**.
- Both instantiate on mono and stereo tracks.
- Editor opens at default size, resizes to the minimum, closes, and reopens.
- Plugin unload and project close do not hang or crash.

### Audio and performance

- Dry audio passes unchanged with no pad held.
- Exercise repeat, reverse, tape stop, gate, all pitch actions, three bands,
  and LoFi.
- Verify overlapping/layered effects and newest-buffer-pad priority.
- Change project tempo and buffer size while processing.
- Confirm rolling history, captured slice, reverse playhead, and tape-stop
  rate displays.

### MIDI

1. Open **View → Virtual MIDI Keyboard**.
2. Select the virtual keyboard as track MIDI input.
3. Arm the track and enable input monitoring.
4. Enable **Send all keyboard input** when typing with another window active.
5. Verify notes `60..75` stopped and playing.
6. Verify pad illumination and Auto-select MIDI.
7. Verify pitch aliases `57..59` and `81..83`.
8. Keep the editor focused and confirm unused key events still reach REAPER.

### Automation and state

- Record and play back pad triggers, effect selection, macros, and performance
  pitch.
- Confirm continuous controls produce clean begin/set/end gestures.
- Save, close, and reopen the project; verify parameters and kit name.
- Reopen the editor and verify selected effect labels/values are correct.

### Kits

- Browse Classic, Glitch Grid, Tape Lab, and Filter & Pitch.
- Confirm each change selects pad 1, releases triggers, and clears captured
  audio.

### Presets

- Use REAPER's preset controls above the embedded editor to save the current
  complete plugin state.
- Change kits and controls, load the saved host preset, and verify all 145
  parameters and the displayed kit name are restored.
- Confirm the embedded editor contains no duplicate LOAD/SAVE buttons and opens
  no platform file dialogs.

## Benchmark

```sh
cargo run --locked -p buffer-uppercut-dsp --release --example benchmark
```

Use at least five like-for-like runs for a controlled comparison. A repeatable
audio-callback regression above 10% blocks sign-off unless explicitly accepted.

## Definition of done

A feature is complete when:

- focused tests cover its behavior and failure cases;
- workspace, Clippy, formatting, and `rt-paranoid` pass;
- advertised formats build;
- screenshots or DAW checks cover visible/host-facing changes;
- authoritative contracts, architecture, roadmap, and ADRs are updated;
- no build products, installed bundles, or caches are committed.
