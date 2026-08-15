# Roadmap

Last updated: 2026-08-15.

This file tracks current product status. Architectural rationale belongs in
`adr/`; durable behavior belongs in `CONTRACTS.md`.

## Current baseline

- TRUCE 6.3 is the sole canonical framework.
- Slint 1.15.1 is the sole editor implementation.
- Product identity is **Buffer Uppercut**, not a preview or comparison build.
- CLAP, VST3, and standalone targets build.
- The framework-neutral `f64` DSP implements every current effect as an
  independent per-slot processor in a deterministic ascending-slot serial chain.
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
- Three deterministic size baselines plus the captured-state baseline exist for
  the Slint editor.
- A manual macOS REAPER smoke test passed for the earlier canonical build.
- Linux, Windows, and macOS CI cover formatting, Clippy, contract tests,
  `rt-paranoid`, CLAP/VST3/standalone builds, and headless validators.
- Archived C++/WRAC state, preset, ID, and source compatibility are explicitly
  out of scope.

## Next: serial release sign-off

- Complete the full REAPER checklist for serial stacking, same-type stages,
  six-stage suspend/restore, release/repress history, and final **Buffer
  Uppercut** identity.
- Verify the direct-key opt-in/default policy in CLAP, VST3, and standalone.
- Verify physical-position mapping on at least one non-US keyboard layout and
  confirm disabled/unmapped keys remain available to the host.
- Measure activation memory at supported sample rates and record controlled CPU
  baselines at active caps of one through six with visualization publishing
  enabled.
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
