# Testing and acceptance

## Test levels

### Fast local check

```sh
cargo test --locked --workspace
```

This runs DSP units, wrapper/MIDI/parameter/editor tests, native kit codec tests,
the canonical serial audio regression corpus, and framework-patch tests.

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

V1 is frozen historical evidence. V2 is the canonical serial expectation:

```sh
(cd contract && shasum -a 256 -c SHA256SUMS)
(cd contract/v2 && shasum -a 256 -c SHA256SUMS)
cargo test -p buffer-uppercut-dsp --test contract
```

The contract harness must execute `contract/v2/*.budsp`. It must not make the
intentional serial architecture pass by rewriting or silently blessing
`contract/*.budsp`. V1 integrity is checked by its original checksum list.

The v2 fixture set includes the retained broad processing scenarios plus focused
coverage for:

- two same-type stages stacking serially;
- the six-stage cap, oldest-active suspension, and most-recently-held restore;
- frozen processor state with configured buffer history continuing during
  suspension;
- release resetting processor state while configured history continues; and
- buffer lookback captured at the slot's own serial chain position.

## Focused serial checks

During implementation, run:

```sh
cargo test --locked -p buffer-uppercut-dsp --lib
cargo test --locked -p buffer-uppercut-kit
cargo test --locked -p buffer-uppercut-dsp --test contract
cargo test --locked -p buffer-uppercut-dsp --features rt-paranoid
```

DSP unit coverage must inspect admission masks and internal lifecycle semantics
that sample fixtures cannot express directly. Kit tests must keep version 1,
16 slots, seven macros, independent repeated same-type configurations, and
non-persistence of momentary held state.

## Slint screenshots

Check all committed sizes:

```sh
cargo truce screenshot --out screenshots/slint-default.png --check --debug --scale 1
BUFFER_UPPERCUT_EDITOR_SIZE=920x620 cargo truce screenshot --out screenshots/slint-minimum.png --check --debug --scale 1
BUFFER_UPPERCUT_EDITOR_SIZE=1440x760 cargo truce screenshot --out screenshots/slint-wide.png --check --debug --scale 1
BUFFER_UPPERCUT_EDITOR_PREVIEW=captured cargo truce screenshot --out screenshots/slint-captured.png --check --debug --scale 1
```

The default, minimum, and wide baselines exercise rolling history. The captured
baseline exercises fixed layout geometry, capture emphasis, and held-slot
hierarchy. Additional state baselines for direct-key-held, MIDI-held, reverse,
suspended, and disabled-macro states remain roadmap work.

A Vinyl preview exercises all seven macro labels and the final selector choice:

```sh
BUFFER_UPPERCUT_EDITOR_PREVIEW=vinyl cargo truce screenshot --out screenshots/slint-vinyl.png --check --debug --scale 1
```

## Focused Vinyl acceptance

Automated DSP tests cover audible pitch movement, stereo timing, wear response,
saturation harmonics, deterministic independent texture, exact clean endpoints,
block partitioning, duplicate serial stages, suspension/restore, reset, and
sample-rate/automation extremes. The wrapper lifecycle test explicitly enters
an `rt-paranoid` section around processing, including six Vinyl stages,
overflow, type changes, and kit reset. Kit tests cover Vinyl round-trip and
rejection of effect values above 12.

In both CLAP and VST3 in REAPER:

- Load Vinyl Cuts and audition its first four pads on drums and sustained tones.
- Sweep all seven macros; check for objectionable transitions and excessive gain.
- Turn Dust and Noise off; confirm silence over silent input and no click tail.
- Set Wet to zero, then all sound controls to zero; confirm clean output after
  smoothing settles. Compare mono and stereo sources.
- Stack duplicate Vinyl pads and six active stages; suspend/restore a seventh.
- Record type/macro automation, save and reopen the project, and check recall.
- Release/repress and change effect type; confirm no stale delayed audio returns.
- Change sample rate, tempo, buffer size, and transport position; check the
  lifecycle rules in `CONTRACTS.md`. Record CPU and perform listening acceptance.

## Vinyl verification evidence (2026-09-09)

- Formatting, all-target/all-feature Clippy, workspace tests, `rt-paranoid`, and
  CLAP/VST3 release builds pass. Both frozen corpus checksum sets pass.
- All five screenshot baselines pass, including Vinyl with all seven controls.
  Existing images changed only at the selector knob to reflect thirteen choices.
- CLAP validator: 42 passed, zero failures/warnings, two skipped.
- REAPER 7.78 on macOS 26.5.2: both new bundles were loaded from the build
  directory in an isolated resource profile. Both expose Vinyl, open their native
  editor, load Vinyl Cuts through the factory arrows, and recall the type and
  all seven macro values after project reload.
- Scripted CLAP writes immediately followed by editor creation were superseded
  before application. Separating parameter application from editor creation
  passes both pre-save checks and reload checks; this combined host sequence
  remains an investigation item in ROADMAP.md.
- Preliminary optimized DSP-only timing with 256-frame blocks: one/six maximum
  Vinyl stages used 0.50%/3.42% of one core at 48 kHz and 3.32%/12.42% at 192 kHz.
  These measurements ran alongside other work and exclude wrapper/visualization
  costs; they are not controlled release CPU acceptance.
- Listening, recorded DAW automation, mono/stereo source acceptance, and the full
  six-stage host performance checklist remain release work.

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
- Activation/sample-rate changes, kit application, and host state restore clear
  every history and reset admission/processor state.
- Transport start, stop, seek, loop, tempo, and position changes preserve held,
  processor, admission, and history state.

### Audio and serial performance

- Dry audio passes unchanged with no slot held.
- Exercise repeat, reverse, tape stop, gate, all pitch actions, three bands, and
  LoFi and Vinyl.
- Hold two gates, two bands, two LoFi stages, and two buffer stages in separate
  trials. Confirm repeated exact types stack instead of replacing one another.
- Press the same configured slots in different orders. Confirm audible stage
  order remains ascending slot number.
- Confirm each stage's wet/dry control is local to its chain input.
- Hold six continuous slots, then press a seventh and eighth. Confirm each new
  request is heard, the oldest admitted active slot suspends, and suspended slots
  remain visibly held.
- Release capacity in stages. Confirm still-held suspended slots restore
  most-recently-held first but resume at ascending slot positions.
- Suspend a buffer stage, feed distinctive audio, then restore it. Confirm its
  processor resumes frozen phase/capture state and its lookback includes audio
  recorded while suspended.
- Release a configured buffer, feed distinctive audio, and press it again.
  Confirm processor state restarts while lookback uses the continuously recorded
  history.
- Place an active filter before a configured buffer slot. Confirm later lookback
  contains the filtered chain-position signal, not raw host input.
- Automate buffer-to-buffer and non-buffer-to-buffer type changes. Confirm
  preserved versus cold history behavior.
- Change project tempo and buffer size while processing.
- Confirm rolling history, selected captured slice, suspended frozen playhead,
  reverse playhead, and tape-stop rate displays.

### Direct computer keys

1. With direct keys disabled in CLAP/VST3, verify `1234/QWER/ASDF/ZXCV` and
   unmapped keys remain available to REAPER.
2. Enable **DIRECT KEYS** and verify the four rows map to slots 1 through 16 by
   physical position.
3. Verify key release ends only the direct-key hold when MIDI, automation, or
   pointer still holds the same slot.
4. Verify repeated key-down events do not create stuck or duplicate releases.
5. Verify pitch-action slots fire once per aggregate released-to-held edge.
6. Test one non-US keyboard layout and confirm the physical positions do not
   move with produced characters.
7. In standalone, verify direct keys are enabled by default.
8. Disable direct keys again and confirm host keyboard behavior returns.

### MIDI

1. Open **View → Virtual MIDI Keyboard**.
2. Select the virtual keyboard as track MIDI input.
3. Arm the track and enable input monitoring.
4. Enable **Send all keyboard input** when typing with another window active.
5. Verify notes `60..75` stopped and playing.
6. Verify slot illumination and Auto-select MIDI.
7. Verify pitch aliases `57..59` and `81..83`.
8. Hold the same note from two MIDI channels and release them separately. The
   slot must remain held until both channels release it.
9. Combine MIDI with direct-key, automation, and pointer holds on one slot;
   release each source independently.
10. Keep the editor focused with direct keys disabled and confirm unused key
    events still reach REAPER.

### Automation and state

- Record and play back slot triggers, effect selection, macros, and performance
  pitch.
- Confirm continuous controls produce clean begin/set/end gestures.
- Save, close, and reopen the project; verify all 145 parameters and kit name.
- Confirm state restoration releases transient holds and clears processor,
  admission, rolling-history, and slot-history state.
- Reopen the editor and verify selected effect labels/values are correct.

### Kits

- Browse Classic, Glitch Grid, Tape Lab, and Filter & Pitch.
- Confirm each change selects slot 1, releases every input-source hold, clears
  admission and every history, and resets processor state.
- Save and reload a kit with two same-type slots using different macros. Confirm
  both configurations survive and the file remains kit version 1.

### Presets

- Use REAPER's preset controls above the embedded editor to save the current
  complete plugin state.
- Change kits and controls, load the saved host preset, and verify all 145
  parameters and the displayed kit name are restored.
- Confirm the embedded editor contains no duplicate LOAD/SAVE buttons and opens
  no platform file dialogs.

## Benchmark and memory

```sh
cargo run --locked -p buffer-uppercut-dsp --release --example benchmark
```

Use at least five like-for-like runs for active caps one through six. A repeatable
audio-callback regression above 10% blocks sign-off unless explicitly accepted.
Record activation memory at common sample rates, including the next-power-of-two
cost of 16 stereo eight-second `f32` histories plus rolling history.

## Definition of done

A feature is complete when:

- focused tests cover its behavior and failure cases;
- workspace, Clippy, formatting, and `rt-paranoid` pass;
- advertised formats build;
- both corpus checksum sets pass and the canonical v2 harness passes;
- screenshots or DAW checks cover visible/host-facing changes;
- authoritative contracts, architecture, roadmap, and ADRs are updated;
- no build products, installed bundles, or caches are committed.
