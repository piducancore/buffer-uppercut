# Roadmap

Last updated: 2026-09-24.

This file tracks current product status. Architectural rationale belongs in
`adr/`; durable behavior belongs in `CONTRACTS.md`.

## Current baseline

- Pitch is one effect with Down, Trigger, and Up roles. A held Trigger runs a
  tempo-preserving grain shifter; action taps accumulate their configured Step
  until the final Trigger releases. Active Shift is transient, read-only UI
  state rather than a host parameter or preset field. Beat Repeat keeps
  independent Slice Pitch.
- Vinyl is effect type 8 with independent wow/flutter delay, wear, drive, dust,
  noise, and wet mix. The fifth factory kit, Vinyl Cuts, provides four textures.
  The selector has nine choices.

- TRUCE 6.3 is the sole canonical framework.
- Slint 1.15.1 is the sole editor implementation. The editor uses a cream and
  charcoal chassis, light aligned keycaps, responsive dials, and acid-yellow
  selection/performance accents; see [visual hierarchy](ARCHITECTURE.md#editor-visual-hierarchy).
- Product identity is **Buffer Uppercut**, not a preview or comparison build.
- CLAP and VST3 are the supported product formats. The standalone target builds
  as a development host and is excluded from distribution.
- The framework-neutral `f64` DSP implements every current effect as an
  independent per-slot processor in a deterministic ascending-slot serial chain.
- Filter uses one bounded nonlinear resonant state-variable processor with
  Low-pass, Band-pass, and High-pass Mode values, plus drive, envelope response,
  deterministic motion, and saturated feedback.
- At most six continuous processors are active. New requests suspend the oldest
  admitted processor, and still-held suspended requests restore
  most-recently-held first.
- Sixteen independent stereo `f32` slot histories cover at least eight seconds
  per channel. Configured buffer history records exact chain-position input even
  while its processor is released or suspended.
- The 16-slot host schema has 144 automatable parameters at stable IDs `1..144`.
  Native `.bupreset` version 3 stores durable slot configuration and a validated
  physical-key map without the transient Active Shift value.
- Pointer and configurable physical-key input send combined pad Trigger host
  gestures. MIDI remains independently held. The default layout is
  `1234/QWER/ASDF/ZXCV`; Learn, Clear, Reset Layout, and explicit swaps are
  implemented. Host recording acceptance remains pending.
- Host/native persistence restores the same durable sounds, name, and key map.
  Recall and activation release momentary Triggers. Native-file import/export
  still has no embedded editor UI.
- Direct keys are opt-in for CLAP/VST3 and enabled by default in the standalone
  development host.
- Host tempo, automation, host-native preset recall, semantic macro values,
  factory kits, the native kit codec, and the stereo performance waveform are
  implemented.
- The frozen experimental `dsp-contract-v1` corpus is preserved unchanged.
  `dsp-contract-v2-serial` preserves the first canonical serial outputs, and
  `dsp-contract-v3-unified-filter` preserves the unified Filter milestone, and
  `dsp-contract-v4-grain-pitch` is the current sound contract.
- Processing is allocation-free after activation according to `rt-paranoid`.
- Three deterministic size baselines plus captured-state and Vinyl baselines
  exist for the Slint editor.
- Earlier serial builds loaded in macOS REAPER with MIDI and focused direct-key
  performance. This evidence does not accept the current mapping/gesture changes.
- Linux, Windows, and macOS CI cover formatting, Clippy, contract tests,
  `rt-paranoid`, CLAP/VST3 builds, the standalone development-host build, and
  headless validators.
- Original project code is MIT licensed; dependencies retain upstream terms.
- A tag/manual workflow prepares unsigned CLAP/VST3 draft prereleases after
  automated checks, validators and reviewed release inputs. Publication is
  deliberate; published assets cannot be overwritten by the workflow.
- Archived C++/WRAC state, preset, ID, and source compatibility are explicitly
  out of scope.
- The Astro product website in `site/` uses GitHub Actions-based Pages
  publishing at `https://piducan.dev/buffer-uppercut/`. The canonical repository
  is public; the download page excludes internal candidates and draft releases.
- Weekend-long Ableton Live use on macOS and Windows completed without a
  reported crash. Preview 1 shipped those two platforms. Preview 2 adds an
  x86-64 Linux package with exact-bundle automated validation but no real Linux
  DAW evidence; current LMMS versions do not natively host CLAP or VST3.

## Next: stable release sign-off

- Preview 4 introduces configurable direct-key layouts and host-recordable
  pointer/direct-key pad gestures for macOS, Windows and Linux unsigned
  packages. Platform and exact-artifact acceptance limitations are recorded in
  [its release notes](releases/0.1.0-preview.4.md).

- Complete [performance-input acceptance](TESTING.md#performance-input-acceptance)
  in Ableton Live VST3 and REAPER VST3/CLAP: recording, replay, save/reopen,
  host automation modes, mapping-only project dirty state, and cleanup. Confirm
  144 parameters with ID 0 absent and released Trigger recall with editor closed.
  Rapid taps within one block can collapse; do not claim lossless capture.
  The [implementation plan](PERFORMANCE-INPUT-PLAN.md) remains pending DAW acceptance.
- Current-build automated checks, five screenshot checks, CLAP/VST3 builds and
  installed CLAP validation pass. Complete VST3 pluginval (not installed locally)
  and installed-host checks before accepting this increment.
- Linux needs a compatible CLAP/VST3 host smoke test before stable promotion;
  LMMS cannot directly exercise the shipped formats. See [release inputs](releases/README.md).

- Finish recorded type/macro automation and filter listening acceptance in
  `TESTING.md`. VST3 knob synchronization and type-default writes pass live host
  checks; VST3/CLAP fresh defaults and custom Wet recall pass. Headless regression
  tests cover previously edited knobs and every Classic kit startup control.
- Finish Pitch listening in CLAP and VST3: repeated Down/Up taps, final-Trigger
  reset, tempo preservation, grain controls, automation recall, serial stacking,
  and worst-case CPU. Automated DSP/wrapper tests, CLAP Validator, installed
  VST3 discovery, native-editor opening, and Classic role/default inspection
  pass.
- Finish Vinyl listening and recorded-automation acceptance, mono/stereo source
  checks, and six-stage host CPU measurements in `TESTING.md`. CLAP/VST3 selector,
  native-editor opening, and project recall of all seven macros pass in REAPER.
- Investigate the CLAP host edge case where scripted parameter writes immediately
  followed by editor creation can be superseded before the host applies them.
  Applied parameters recall correctly after the host processes its queue.

- Complete the full REAPER checklist for serial stacking, same-type stages,
  six-stage suspend/restore, release/repress history, and final **Buffer
  Uppercut** identity.
- Verify the direct-key opt-in/default policy in CLAP, VST3, and the standalone
  development host.
- Verify physical-position mapping on at least one non-US keyboard layout and
  confirm disabled/unmapped keys remain available to the host.
- Measure activation memory at supported sample rates and record controlled CPU
  baselines at active caps of one through six with visualization publishing
  enabled, including worst-case nonlinear filters.
- Complete focused listening acceptance for the nonlinear Filter's LP/BP/HP
  modes, including resonance, drive, envelope, motion, automation sweeps, and
  six-stage serial stacking.
- Run pluginval GUI tests in an interactive desktop session.
- Add deterministic screenshot states for direct-key-held, MIDI-held, reverse,
  suspended, and disabled-macro views. Captured performance has a committed
  baseline.

## Candidate product work

- Sample-accurate event segmentation.
- Ordered rapid-event handling before claiming lossless sub-block pad taps.
- Improved accessibility and general keyboard navigation outside performance
  mode.
- Additional factory kits and kit-management workflow.
- Expanded visualization or spectrum analysis.
- Preset browser and tagging.
- Add macOS and Windows signing and macOS notarization to the CLAP/VST3 release
  workflow after credentials are available; unsigned previews remain labeled.
- Define standalone audio-input, monitoring, and no-input behavior before
  reconsidering it as a distributed product.
- Additional plugin formats after CI and real-host validation.

## Explicitly not planned

- Migration from archived C++ or WRAC project state.
- Cross-framework preset interchange.
- Maintaining old experimental product IDs or parameter layouts.
- Replacing the 16-slot schema solely for the serial architecture.
- WebView or egui editor variants in this repository.
- Sharing DSP implementation code with an archived port.

## Keeping this file useful

Update the date and sections whenever a milestone begins or completes. Remove
finished tasks instead of accumulating a historical changelog; Git already
stores history.
