# Controlled WRAC/TRUCE comparison

Compare this repository with `buffer-uppercut-wrac` at the same preview
milestone.

| Concern | WRAC preview | TRUCE preview |
| --- | --- | --- |
| Language | Rust | Rust |
| GUI | HTML/CSS/TypeScript in WebView | native egui |
| GUI↔plugin bridge | explicit commands/events and serialization | typed parameter bindings |
| Parameters | explicit host parameter specification | derived nested parameter structs |
| Formats in preview | CLAP, VST3, standalone | CLAP, VST3, standalone |
| MIDI and transport | wired | wired |
| State | host parameter state | host parameter state |
| DSP at this milestone | pass-through | pass-through |

## What to measure

- clean build time and incremental edit/build time
- code needed for one new parameter and one new UI control
- plugin binary and bundle size
- standalone startup time and editor responsiveness
- automation gesture behavior in a DAW
- state save/restore
- MIDI and transport consistency
- validator and host compatibility
- debugging quality when something fails

## Current developer-experience hypothesis

TRUCE + egui should be more concise for typed plugin UI work because parameter
widgets call the host automation protocol directly and there is no web command
surface to maintain. WRAC should remain stronger when browser-native layout,
styling, accessibility, or reuse of a web application matters more.

The native path is not automatically smaller or more portable in every sense:
egui brings a GPU rendering stack, and native-window behavior still needs host
testing. The comparison should be decided after the same DSP engine and audio
fixtures run behind both wrappers.

## Fair next milestone

1. extract or port the same framework-light DSP engine into each repository
2. feed identical deterministic audio/MIDI/transport fixtures
3. compare rendered audio within an agreed tolerance
4. test state migration by stable parameter ID
5. validate both builds in the same DAWs

Avoid evaluating DSP quality until that milestone is complete.
