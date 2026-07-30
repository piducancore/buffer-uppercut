# Architecture

## System overview

Buffer Uppercut separates product behavior from framework and UI integration:

```text
Host audio / MIDI / transport / automation
                    |
                    v
          TRUCE PluginLogic64
              src/lib.rs
          /         |          \
         v          v           v
  src/midi.rs  src/params.rs  preallocated wrapper I/O
         \          |           /
          \         v          /
           ---> DSP Engine <---
              dsp/src/lib.rs
                    |
                    v
       atomic visualization snapshot
             src/params.rs
                    |
                    v
       Slint bindings and components
       src/editor.rs + ui/main.slint
```

The `dsp` and `kit` crates have no TRUCE, Slint, host, filesystem, or wrapper
dependencies.

## Repository map

| Path | Owns | Must not own |
| --- | --- | --- |
| `dsp/` | audio engine, effects, performance state, control metadata, visualization extraction | TRUCE, UI, files, host events |
| `kit/` | factory-kit definitions and `.bupreset` codec | file dialogs, automation, host state |
| `src/lib.rs` | plugin lifecycle, processing, host transport/events, parameter-to-DSP translation | visual layout, file dialogs |
| `src/params.rs` | parameter schema, persisted kit name, atomic MIDI/waveform publication | effect processing |
| `src/midi.rs` | note mapping and per-channel held masks | parameters, DSP |
| `src/editor.rs` | host automation gestures, kit file I/O, waveform path construction | audio processing |
| `ui/` | Slint components, visual hierarchy, responsive layout | plugin or DSP logic |
| `vendor/` | documented narrow framework backports | product features |

## Audio processing flow

`PluginLogic64::reset` runs at activation and sample-rate/block-size changes. It
sizes all wrapper scratch buffers and resets the DSP ring buffers.

For every process block:

1. Read host tempo.
2. Apply MIDI events to 16 independent channel-held masks.
3. Aggregate MIDI and automatable trigger parameters into a
   `PerformanceState`.
4. Apply pitch actions once per aggregate released-to-held transition.
5. Copy host input into activation-sized planar `f64` scratch.
6. Process through `Engine`.
7. Publish a 256-bin visualization snapshot when its 30 Hz interval expires.
8. Flush subnormal output and copy it to the host.

Events remain block-boundary events. Sample-accurate event segmentation is not
implemented.

## Parameters and host state

`BufferUppercutParams` is the host-facing source of truth. It contains 145
automatable parameters. `kit_name` is `#[persist]` metadata and does not change
the parameter count. MIDI illumination, last MIDI press, kit reset sequence,
and waveform frames are `#[skip]` runtime fields.

The DSP never mutates the parameter store. Pitch actions emit a host
`ParamChange` event, allowing each wrapper to record the change correctly.

## Kit flow

Factory navigation and file loading run on the UI thread:

```text
factory selection or .bupreset file
        -> validated Kit
        -> normalized host automation writes
        -> persisted kit name
        -> atomic reset sequence
        -> audio-thread transient clear at next block
```

File access never occurs on the audio thread. Kit application releases triggers
and clears captured audio without resizing DSP buffers.

## Visualization flow

`Engine::fill_visualization` reads preallocated audio history into 256 stereo
min/max bins. The wrapper publishes those bins through a sequence-checked
atomic bridge. The UI accepts only a complete sequence; otherwise it retains
the previous frame. Slint creates dynamic path geometry on the UI thread.

Rolling mode displays the newest four beats. A held buffer effect displays its
captured slice, playhead, direction, playback rate, active effect, and duration.

## Concurrency model

| Data | Writer | Reader | Synchronization |
| --- | --- | --- | --- |
| host parameters | host/UI | audio/UI | TRUCE atomic parameter storage |
| MIDI held mask | audio | UI | atomics |
| waveform frame | audio | UI | sequence-checked atomics |
| kit reset sequence | UI | audio | atomic counter |
| kit name | UI/host restore | UI/host save | `RwLock`, never read in `process` |
| DSP engine | audio lifecycle | audio lifecycle | exclusive `DspState` access |

## Framework patches

`vendor/truce-clap` contains a state-rescan backport.
`vendor/truce-slint` contains keyboard passthrough and related integration
fixes. Each patch directory documents its provenance and removal condition.
Product behavior must not be implemented by expanding these patches.
