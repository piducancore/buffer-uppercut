# Architecture

## System overview

Buffer Uppercut separates product behavior from framework and UI integration:

```text
Host audio / MIDI / transport / automation
Computer-key and pointer performance input
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
| `dsp/` | serial audio engine, per-slot runtime state and history, admission state, effect behavior, control metadata, visualization extraction | TRUCE, UI, files, host events |
| `kit/` | factory-kit definitions and `.bupreset` codec | file dialogs, automation, host state |
| `src/lib.rs` | plugin lifecycle, processing, host transport/events, input aggregation, parameter-to-DSP translation | visual layout, file dialogs |
| `src/params.rs` | parameter schema, persisted editor metadata, atomic MIDI/admission/waveform publication | effect processing |
| `src/midi.rs` | note mapping and independent per-channel held masks | parameters, DSP |
| `src/editor.rs` | host automation gestures, direct-key adapter, factory-kit application, waveform path construction | audio processing, file dialogs |
| `ui/` | Slint components, visual hierarchy, responsive layout | plugin or DSP logic |
| `vendor/` | documented narrow framework backports | product behavior |

## Audio processing flow

`PluginLogic64::reset` runs at activation and sample-rate/block-size changes. It
sizes all wrapper scratch, the rolling visualization history, and one independent
stereo history for each of the 16 performance slots. Every slot history uses
`f32` storage and covers at least eight seconds per channel at the active sample
rate. Serial effect calculations and wrapper audio remain `f64`.

For every process block:

1. Read host tempo.
2. Apply MIDI events to 16 independent channel-held masks.
3. Aggregate direct-key, MIDI, automatable trigger, and pointer holds into one
   held state per slot. Releasing one source does not release another source's
   hold.
4. Apply pitch actions once per aggregate released-to-held transition. Pitch
   actions do not enter the continuous audio chain.
5. Copy host input into activation-sized planar `f64` scratch.
6. Resolve held, active, and suspended continuous slots under the six-processor
   cap.
7. Process active stages in ascending slot order. Each stage receives the prior
   stage's output and uses that signal as its local wet/dry reference.
8. Record every configured buffer slot's chain-position input into its own
   history, whether the slot is released, active, or suspended.
9. Publish a 256-bin visualization snapshot when its 30 Hz interval expires.
10. Flush subnormal output and copy it to the host.

Events remain block-boundary events. Sample-accurate event segmentation is not
implemented.

## Serial slot and admission model

Every held continuous effect is an independent stage, including two instances of
the same exact effect type. The audible chain always follows ascending slot
number, never press order or restoration order.

At most six continuous processors are active. A new request at capacity suspends
the oldest admitted active slot and becomes active immediately. A suspended
slot remains held but is not processed. When capacity returns, still-held
suspended requests are restored most-recently-held first, after which the audible
chain is again sorted by slot number.

Suspension freezes mutable processor state such as phase, filter state, gate
gain, LoFi hold state, and a buffer effect's capture/playback state. It does not
freeze configured buffer history. That history continues recording the exact
signal reaching the slot's chain position and continues advancing, so lookback
is current when the slot returns. Aggregate release removes the slot from
admission and resets its processor state, but a configured buffer history keeps
recording and is available to the next press.

Pitch Down, Pitch Reset, and Pitch Up are edge-triggered host parameter actions.
Off and pitch actions do not consume the continuous-processor cap.

## Buffer history and type changes

A buffer-configured slot records before its own processor is applied. Therefore
its ring contains the output of every active lower-numbered stage and none of the
higher-numbered stages. This is the exact input the slot would receive in the
serial chain.

Changing buffer type within a slot preserves that slot's history while resetting
capture/playback processor state. Changing from a non-buffer type to a buffer
type clears that slot's history and begins cold because the engine cannot
fabricate earlier chain-position input. Histories are allocated and cleared only
through lifecycle/reset paths, never resized in `process`.

## Computer-key flow

The performance layout follows physical key positions:

```text
1 2 3 4
Q W E R
A S D F
Z X C V
```

These positions map to slots 1 through 16 row by row. The mapping is physical,
not dependent on the character produced by the active keyboard layout. The
editor exposes it as **DIRECT KEYS**. It is opt-in for CLAP and VST3 instances
so DAW shortcuts and host virtual MIDI keyboards remain available by default.
Standalone mode enables it by default. MIDI, automation, and pointer input
remain available in every target. Framework support for physical identity
remains generic; product mapping belongs in the Buffer Uppercut integration
layer, not `vendor/truce-slint`.

## Parameters and host state

`BufferUppercutParams` is the host-facing source of truth. It contains 145
automatable parameters. `kit_name` is persisted metadata and does not change the
parameter count. Direct-key enablement, MIDI illumination, input held-state
bridges, admission status, kit reset sequence, and waveform frames are runtime
fields.

The DSP never mutates the parameter store. Pitch actions emit a host
`ParamChange` event, allowing each wrapper to record the change correctly.

## Kit and state flow

Factory navigation and kit application run on the UI thread:

```text
factory selection
        -> validated 16-slot Kit v1
        -> normalized host automation writes
        -> persisted kit name
        -> release all input sources
        -> atomic reset sequence
        -> audio-thread transient/history clear at next block
```

Kit application, plugin/DSP activation, and host state restoration release all
held slots, reset admission and per-slot processor state, and clear rolling and
per-slot histories without resizing prepared storage. Transport start, stop,
seek, tempo, and position changes preserve histories, processor state, and held
or admission state.

Complete plugin preset loading and saving belongs to the host-native preset UI
supplied by the TRUCE wrappers. No platform file dialog is launched from the
embedded editor.

## Visualization flow

`Engine::fill_visualization` reads preallocated `f32` history into 256 stereo
min/max bins. The wrapper publishes those bins through a sequence-checked atomic
bridge. The UI accepts only a complete sequence; otherwise it retains the
previous frame. Slint creates dynamic path geometry on the UI thread.

Rolling mode displays the newest four beats. For captured mode, the
editor-selected held buffer slot wins even when suspended. Its processor
capture/playhead state is frozen while suspended, although its configured ring
continues recording. If the selected slot does not qualify, the lowest-numbered
active buffer slot wins. If neither exists, rolling history is displayed.

## Concurrency model

| Data | Writer | Reader | Synchronization |
| --- | --- | --- | --- |
| host parameters | host/UI | audio/UI | TRUCE atomic parameter storage |
| MIDI held masks | audio | audio/UI | per-channel state plus atomics |
| direct-key held mask | UI input adapter | audio/UI | bounded atomic bridge |
| admission status | audio | UI | atomics |
| waveform frame | audio | UI | sequence-checked atomics |
| kit reset sequence | UI | audio | atomic counter |
| kit name/editor settings | UI/host restore | UI/host save | non-audio synchronization, never read in `process` |
| DSP engine | audio lifecycle | audio lifecycle | exclusive `DspState` access |

## Framework patches

`vendor/truce-clap` contains a state-rescan backport.
`vendor/truce-slint` contains window-handle, scale reconciliation, keyboard
passthrough/physical-key capability, and renderer fixes required by the native
editor. Each patch has a `PATCH.md` with its upstream boundary. Remove a patch
when the version-locked upstream dependency provides equivalent behavior.
