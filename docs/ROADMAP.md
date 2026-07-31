# Roadmap

Last updated: 2026-07-30.

This file tracks current product status. Architectural rationale belongs in
`adr/`; durable behavior belongs in `CONTRACTS.md`.

## Current baseline

- TRUCE 6.3 is the sole canonical framework.
- Slint 1.15.1 is the sole editor implementation.
- Product identity is **Buffer Uppercut**, not a preview or comparison build.
- CLAP, VST3, and standalone targets build.
- The framework-neutral `f64` DSP implements every current pad category.
- MIDI, host tempo, automation, host-native preset recall, semantic macro
  values, factory kits, the versioned `.bupreset` codec, and the stereo
  performance waveform are implemented.
- Processing is allocation-free after activation according to `rt-paranoid`.
- Three deterministic size baselines exist for the Slint editor.
- A manual macOS REAPER smoke test passed for the canonical build.
- Archived C++/WRAC state, preset, ID, and source compatibility are explicitly
  out of scope.

## Next: canonical-product sign-off

- Complete the full REAPER checklist for the final **Buffer Uppercut** identity.
- Run pluginval GUI tests in an interactive desktop session.
- Add deterministic screenshot states for MIDI-held, reverse, and
  disabled-macro views. Captured performance has a committed baseline.
- Record controlled benchmark baselines with visualization publishing enabled.
- Confirm Linux, Windows, and macOS CI for CLAP, VST3, standalone, validators,
  and `rt-paranoid`.
- Decide repository/remote rename from `buffer-uppercut-truce` as an operational
  GitHub task; it does not affect plugin identity.

## Candidate product work

- Sample-accurate event segmentation.
- Improved accessibility and keyboard navigation.
- Additional factory kits and kit-management workflow.
- Expanded visualization or spectrum analysis.
- Preset browser and tagging.
- Release signing, notarization, packaging, and update distribution.
- Additional plugin formats after CI and real-host validation.

## Explicitly not planned

- Migration from archived C++ or WRAC project state.
- Cross-framework preset interchange.
- Maintaining old experimental product IDs or parameter layouts.
- WebView or egui editor variants in this repository.
- Sharing DSP implementation code with an archived port.

## Keeping this file useful

Update the date and sections whenever a milestone begins or completes. Remove
finished tasks instead of accumulating a historical changelog; Git already
stores history.
