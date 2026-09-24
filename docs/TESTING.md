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

V1–V3 are frozen historical evidence. V4 is the current expectation:

```sh
(cd contract && shasum -a 256 -c SHA256SUMS)
(cd contract/v2 && shasum -a 256 -c SHA256SUMS)
(cd contract/v3 && shasum -a 256 -c SHA256SUMS)
(cd contract/v4 && shasum -a 256 -c SHA256SUMS)
cargo test -p buffer-uppercut-dsp --test contract
```

The contract harness must execute `contract/v4/*.budsp`. It must not make the
intentional serial architecture pass by rewriting or silently blessing
earlier corpora. V1–V3 integrity is checked by their checksum lists.

The v4 fixture set retains the broad processing and serial scenarios, migrates
the held grain Pitch schema, and includes focused coverage for:

- two same-type stages stacking serially;
- the six-stage cap, oldest-active suspension, and most-recently-held restore;
- frozen processor state with configured buffer history continuing during
  suspension;
- release resetting processor state while configured history continues; and
- buffer lookback captured at the slot's own serial chain position.

## Focused Pitch acceptance

Automated tests cover role quantization, configured action steps and clamping,
Trigger admission, action-role exclusion, exact dry output at zero shift,
finite shifted output, expected octave direction, wrapper edge handling, and
reset to zero when the final Trigger releases. The v4 corpus also verifies that
Beat Repeat Slice Pitch is independent of Active Shift.

In both CLAP and VST3 in REAPER:

- Hold the Classic D Trigger and tap S/F repeatedly. Confirm each new press
  moves by the action pad's Step and holding a key does not retrigger.
- Release D and confirm Active Shift immediately returns to zero and the dry
  timing resumes. Repeat with two Trigger pads and confirm only the final release
  resets it.
- Change Step on Down and Up independently and verify their increments.
- Sweep Grain, Texture, Smooth, Feedback, and Wet on a sustained tone and drums;
  confirm tempo and event length remain stable across positive and negative
  shifts.
- Confirm action roles never suspend a continuous stage, while multiple Trigger
  roles stack as distinct serial processors under the six-stage cap.
- Automate roles and all macros, save/reopen, and verify exact recall. Confirm
  Beat Repeat Slice Pitch remains local when Active Shift changes.

## Pitch verification evidence (2026-09-09)

- Formatting, all-target/all-feature Clippy, workspace tests, `rt-paranoid`, the
  v1–v4 checksum sets, and CLAP/VST3 release builds pass.
- The installed bundle executables match the release-build hashes. CLAP
  Validator reports 42 passed, zero failures or warnings, and two skipped.
- All five screenshot baselines pass. The Classic view shows S/D/F as Pitch −,
  Pitch, and Pitch +, a read-only Active Shift indicator, and the distinct yellow
  type control labeled **TYPE RESETS**.
- REAPER 7.78 loads the installed VST3 and opens its native editor. Reloading
  Classic exposes the three Pitch roles; the Trigger reports Step 1 st, Grain
  48 ms, Texture 20%, Smooth 18 ms, Feedback 0%, and Wet 100%.
- Live listening, multi-Trigger interaction, recorded automation, and project
  recall for the new Pitch roles remain on the release checklist above.

## Focused serial checks

During implementation, run:

```sh
cargo test --locked -p buffer-uppercut-dsp --lib
cargo test --locked -p buffer-uppercut-kit
cargo test --locked -p buffer-uppercut-dsp --test contract
cargo test --locked -p buffer-uppercut-dsp --features rt-paranoid
```

DSP unit coverage must inspect admission masks and internal lifecycle semantics
that sample fixtures cannot express directly. Kit tests must keep version 3,
16 slots, seven macros, independent repeated same-type configurations, and
non-persistence of momentary held state, plus validated key-map round-trips.

## Performance-input acceptance

Status: implemented; real-DAW acceptance remains pending.
Earlier editor/Pitch/preview evidence below does not verify this increment.

### Current implementation evidence (2026-09-24)

- Formatting, all-target/all-feature Clippy, workspace tests, `rt-paranoid`,
  and CLAP/VST3 release builds pass. Host snapshot tests cover save before
  processing, durable map/sound recall, suppressed saved holds, and byte-identical
  resaving. The constant lifecycle marker publishes at construction.
- All five size/captured/Vinyl screenshots were visually reviewed, updated for
  mapping controls, and pass exact baseline checks. Minimum-size pad text was
  adjusted to avoid clipping.
- V1–V4 corpus checksums pass unchanged. Installed CLAP passes CLAP Validator.
- CLAP and VST3 are installed, with executable SHA-256 hashes matching their
  built bundles: CLAP `a8a6fc32c08c2b645f23afc14ed3ccef467509fc323a5146254f392cb7e4b3e2`;
  VST3 `602362ce5bd8753d86ff3d779d1f5c6f6389ca5be12f676e18e095b5689e36ee`.
  VST3 pluginval could not run because the validator
  is not installed. Interactive REAPER access timed out; no new real-host
  recording/replay, mapping-dirty, or listening acceptance is claimed.
- Released recall suppresses restored high Triggers rather than rewriting host
  values. Rewriting those values failed host state-reproducibility validation;
  suppression preserves the host state while preventing implicit resumed holds.

### Remaining host checklist

- Ableton Live VST3: enable Arrangement Automation Arm, record key and pointer
  gestures, inspect Trigger 1/0 edges, replay without live input, then save/reopen
  and replay. Test Session automation separately and record its settings.
- REAPER VST3 and CLAP: test recording/replay in write and touch modes, plus
  read/latch behavior and live edits over an existing envelope. Identify host
  version, OS, format, installed binary hash, and settings with each result.
- Hold a key and the same pointer pad together; either individual release must
  preserve the remaining local ownership. MIDI must remain independently held.
  Record repeated Pitch Down/Up taps while a Trigger is held, including multiple
  Pitch Triggers and final-Trigger release.
- Stop recording while held, release after stopping, and restart playback.
  Exercise focus loss, close/reopen, Direct Keys disable, factory recall, host
  recall, and remapping while held. Check balanced gestures and no stale release
  affecting a subsequent press.
- Measure very short taps at several buffer sizes. Press/release within one
  process block may collapse to released, including Pitch actions. Keep lossless
  rapid-tap capture unaccepted until ordered event handling is implemented and
  tested; emitting host edits alone does not prove capture or audible playback.
- Learn an unused physical key, cancel learning, clear a binding, explicitly
  swap an occupied assignment, and reset the layout. Learning must not play a
  pad; repeats must not activate a newly assigned key until a fresh press.
  Verify physical-position labels on a non-US layout and disabled/unmapped
  passthrough. Inspect mapping controls at minimum/default/wide sizes.
- Change only a mapping in a saved project and check that the host marks the
  project modified. Save/reopen and compare map, kit name, all sound parameters,
  and labels. Factory recall must restore its authored map.
- Save while Trigger is held, then recall with editor open and closed. All
  audible holds and Active Shift must start released/zero even if host Trigger
  values remain high for state reproducibility; subsequent timeline automation
  must still play. Confirm exactly 144 host parameters and no ID 0.
- Compare native v3 codec and host state round-trips for the same sound values,
  name, and custom map. Reject duplicate/unsupported mappings and older native
  versions without partial map publication. The native-file codec has no
  embedded import/export UI; use code-level adapters for that comparison.
- Complete required checks, all committed screenshot checks, validators,
  installation/hash verification, and installed-host testing for this build.

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

## Editor redesign verification (2026-09-23)

The preview 3 editor has passing default, minimum, wide, captured-cell and Vinyl
screenshot baselines. The redesign passed local formatting, all-feature Clippy,
workspace tests, `rt-paranoid`, CLAP/VST3 builds, and CLAP Validator (42 passed,
zero failures or warnings, two skipped).

Both formats opened the redesigned native editor in macOS REAPER 7.78.
Pointer selection and macro dragging passed in both. VST3 additionally verified
that a Wet drag starts from the newly selected pad's current value and that
changing Filter to Vinyl loads all seven defaults. CLAP factory navigation and
the direct-key toggle updated correctly. These local checks do not establish
exact release-ZIP acceptance on every platform or complete the broader release
checklists below.

## Focused Vinyl acceptance

Automated DSP tests cover audible pitch movement, stereo timing, wear response,
saturation harmonics, deterministic independent texture, exact clean endpoints,
block partitioning, duplicate serial stages, suspension/restore, reset, and
sample-rate/automation extremes. The wrapper lifecycle test explicitly enters
an `rt-paranoid` section around processing, including six Vinyl stages,
overflow, type changes, and kit reset. Kit tests cover Vinyl round-trip and
rejection of effect values above 8.

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
  Existing images changed only at the selector knob to reflect the then-current
  choices.
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

### Effect defaults and knob synchronization

- On a fresh instance, trigger LP, BP, and HP with direct keys and confirm each
  changes the audio at 100% Wet. Loading Classic must retain these defaults.
- Drag Wet on one pad, select another pad, and confirm the dial and numeric value
  agree. The next drag must start at the selected pad's value without jumping.
- Repeat after editing the type knob and after preset
  recall with the editor open.
- Change Beat Repeat to Reverse, Filter, and Vinyl. Confirm all seven macros
  receive the new defaults, the held pad remains usable, and repeated pointer
  motion within one discrete type does not reload defaults.
- On Filter, sweep Mode and confirm it snaps to Low-pass, Band-pass, and
  High-pass while the other six control meanings stay fixed.
- Record the type gesture in the host, verify the type and seven macro lanes,
  and confirm all touched gestures end on release. Reopen the project and check
  exact recall of customized settings.

Verification evidence (2026-09-09): formatting, Clippy, workspace tests,
`rt-paranoid`, CLAP/VST3 builds, and all five screenshot checks pass. The
headless pointer regression reproduces the stale drag origin before the binding
fix and passes afterward. Every fresh host control matches the Classic kit.
Installed bundle hashes match the corresponding release builds.

In an isolated REAPER 7.78 session, both VST3 and CLAP expose pads 13–15 as the
same Filter type with LP/BP/HP Mode values, true 100% Wet defaults, and Vinyl as
the final selector choice. Both native editors open. Saving and reopening the
project recalls a changed HP Mode, Frequency, and Wet value in both formats.
Earlier live VST3 interaction also verified that changing pads through the same
Wet widget starts from each pad's current value and that editor type selection
loads all seven defaults. Recorded compound automation and listening acceptance
remain on the checklist.

## Ableton Live endurance evidence

User-reported acceptance (2026-09-14): Buffer Uppercut was used throughout a
weekend in Ableton Live on both macOS and Windows without a crash. The plugin
loaded and was actively played in real music-making sessions. This supports
including macOS and Windows in the first unsigned preview.

This evidence applies to the tested development builds, not yet to the final
ZIPs produced from a preview tag. Before publishing, install each exact draft
package in Ableton Live on its target OS and confirm discovery, audio pass-through,
pad operation and clean unload. The report does not establish Linux host
compatibility, exhaustive CLAP acceptance, every checklist item below, or a
stable-release claim.

## Linux preview evidence

Preview 2 adds an x86-64 Linux package built on Ubuntu 22.04. The exact tagged
CLAP and VST3 bundles must pass CLAP Validator and pluginval on the Linux release
runner before the draft is created. No real Linux DAW smoke test has been
recorded, so the preview accepts and discloses that limitation.

LMMS cannot serve as that smoke-test host because current LMMS releases do not
natively host CLAP or VST3. Test discovery, audio pass-through, editor opening,
MIDI/pointer triggering, project reload and clean unload in a compatible Linux
host before making any Linux compatibility claim.

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
- Exercise repeat, reverse, tape stop, gate, Pitch Trigger with Down/Up actions,
  all three Filter modes, LoFi, and Vinyl.
- Hold two gates, two Filters, two LoFi stages, and two buffer stages in separate
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
5. Hold a Pitch Trigger and verify Down/Up slots fire once per aggregate
   released-to-held edge; release the final Trigger and verify reset to zero.
6. Test one non-US keyboard layout and confirm the physical positions do not
   move with produced characters.
7. In the standalone development host, verify direct keys are enabled by default.
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

- Record and play back slot triggers, effect selection, and macros. Confirm
  Active Shift does not appear as an automation parameter.
- Confirm continuous controls produce clean begin/set/end gestures.
- Save, close, and reopen the project; verify durable sound values, kit name,
  and key map. Momentary Trigger values recall released.
- Confirm state restoration releases transient holds and clears processor,
  admission, rolling-history, and slot-history state.
- Reopen the editor and verify selected effect labels/values are correct.

### Kits

- Browse Classic, Glitch Grid, Tape Lab, and Filter & Pitch.
- Confirm each change selects slot 1, releases every input-source hold, clears
  admission and every history, and resets processor state.
- Save and reload a kit with two same-type slots using different macros. Confirm
  both configurations and a custom map survive native codec round-trip at
  version 3. This is a codec check, not an embedded import/export workflow.

### Presets

- Use REAPER's preset controls above the embedded editor to save the current
  complete plugin state.
- Change kits and controls, load the saved host preset, and verify durable sound
  values, key map, and displayed kit name are restored; all Triggers start released.
- Confirm the embedded editor contains no duplicate LOAD/SAVE buttons and opens
  no platform file dialogs.

## Benchmark and memory

```sh
cargo run --locked -p buffer-uppercut-dsp --release --example benchmark
```

Use at least five like-for-like runs for active caps one through six. A repeatable
audio-callback regression above 10% blocks sign-off unless explicitly accepted.
Record activation memory at common sample rates, including the next-power-of-two
cost of 16 stereo eight-second `f32` histories, 16 bounded grain Pitch delays,
16 Vinyl delays, and rolling history.

## Definition of done

A feature is complete when:

- focused tests cover its behavior and failure cases;
- workspace, Clippy, formatting, and `rt-paranoid` pass;
- advertised formats build;
- all corpus checksum sets pass and the canonical v4 harness passes;
- screenshots or DAW checks cover visible/host-facing changes;
- authoritative contracts, architecture, roadmap, and ADRs are updated;
- no build products, installed bundles, or caches are committed.
