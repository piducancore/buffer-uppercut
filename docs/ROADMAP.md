# Roadmap

Last updated: 2026-09-09.

This file tracks current product status. Architectural rationale belongs in
`adr/`; durable behavior belongs in `CONTRACTS.md`.

## Current baseline

- Vinyl is effect type 12 with independent wow/flutter delay, wear, drive, dust,
  noise, and wet mix. The fifth factory kit, Vinyl Cuts, provides four textures.
  The selector has 13 choices; the 145 IDs and kit v1 layout remain unchanged.

- TRUCE 6.3 is the sole canonical framework.
- Slint 1.15.1 is the sole editor implementation.
- Product identity is **Buffer Uppercut**, not a preview or comparison build.
- CLAP, VST3, and standalone targets build.
- The framework-neutral `f64` DSP implements every current effect as an
  independent per-slot processor in a deterministic ascending-slot serial chain.
- Low Band, Mid Band, and High Band use bounded nonlinear resonant state-variable
  filters with drive, envelope response, deterministic motion, and saturated
  feedback under the existing seven-macro slot schema.
- At most six continuous processors are active. New requests suspend the oldest
  admitted processor, and still-held suspended requests restore
  most-recently-held first.
- Sixteen independent stereo `f32` slot histories cover at least eight seconds
  per channel. Configured buffer history records exact chain-position input even
  while its processor is released or suspended.
- The 16-slot, 145-parameter host schema and `.bupreset` kit version 1 remain
  unchanged.
- MIDI, automation, pointer input, and the physical `1234/QWER/ASDF/ZXCV`
  performance layout aggregate without one source releasing another.
- Direct keys are opt-in for CLAP/VST3 and enabled by default in standalone.
- Host tempo, automation, host-native preset recall, semantic macro values,
  factory kits, the native kit codec, and the stereo performance waveform are
  implemented.
- The frozen experimental `dsp-contract-v1` corpus is preserved unchanged.
  `dsp-contract-v2-serial` owns the canonical serial outputs and new serial,
  cap, history, and chain-position scenarios.
- Processing is allocation-free after activation according to `rt-paranoid`.
- Three deterministic size baselines plus captured-state and Vinyl baselines
  exist for the Slint editor.
- The current serial build loads in macOS REAPER; MIDI and focused direct-key
  performance have been confirmed in the installed plugin.
- Linux, Windows, and macOS CI cover formatting, Clippy, contract tests,
  `rt-paranoid`, CLAP/VST3/standalone builds, and headless validators.
- Archived C++/WRAC state, preset, ID, and source compatibility are explicitly
  out of scope.

## Next: serial release sign-off

- Finish Vinyl listening and recorded-automation acceptance, mono/stereo source
  checks, and six-stage host CPU measurements in `TESTING.md`. CLAP/VST3 selector,
  native-editor opening, and project recall of all seven macros pass in REAPER.
- Investigate the CLAP host edge case where scripted parameter writes immediately
  followed by editor creation can be superseded before the host applies them.
  Applied parameters recall correctly after the host processes its queue.

- Complete the full REAPER checklist for serial stacking, same-type stages,
  six-stage suspend/restore, release/repress history, and final **Buffer
  Uppercut** identity.
- Verify the direct-key opt-in/default policy in CLAP, VST3, and standalone.
- Verify physical-position mapping on at least one non-US keyboard layout and
  confirm disabled/unmapped keys remain available to the host.
- Measure activation memory at supported sample rates and record controlled CPU
  baselines at active caps of one through six with visualization publishing
  enabled, including worst-case nonlinear filters.
- Complete focused listening acceptance for the nonlinear Low/Mid/High filters,
  including resonance, drive, envelope, motion, feedback, automation sweeps, and
  six-stage serial stacking.
- Run pluginval GUI tests in an interactive desktop session.
- Add deterministic screenshot states for direct-key-held, MIDI-held, reverse,
  suspended, and disabled-macro views. Captured performance has a committed
  baseline.

## Candidate product work

- Sample-accurate event segmentation.
- User-remappable direct-key layouts after the physical default is validated.
- Improved accessibility and general keyboard navigation outside performance
  mode.
- Additional factory kits and kit-management workflow.
- Expanded visualization or spectrum analysis.
- Preset browser and tagging.
- Release signing, notarization, packaging, and update distribution.
- Additional plugin formats after CI and real-host validation.

## Explicitly not planned

- Migration from archived C++ or WRAC project state.
- Cross-framework preset interchange.
- Maintaining old experimental product IDs or parameter layouts.
- Replacing the 16-slot or kit v1 schema solely for the serial architecture.
- WebView or egui editor variants in this repository.
- Sharing DSP implementation code with an archived port.

## Keeping this file useful

Update the date and sections whenever a milestone begins or completes. Remove
finished tasks instead of accumulating a historical changelog; Git already
stores history.
