# Product contracts

These are the canonical contracts of the Rust/TRUCE product. They are not
compatibility promises to archived experiments. Before release they may change
through an explicit code, test, documentation, and ADR update.

## Product identity

- Display name: `Buffer Uppercut`
- Vendor: `PiducanCore`
- Package/crate: `buffer-uppercut`
- Bundle slug: `buffer-uppercut`
- Formats currently built: CLAP, VST3, standalone
- Plugin kind: stereo/mono audio effect with MIDI input

Identity is defined in `truce.toml`; do not duplicate it in source code.

## Parameter and slot schema

There are exactly 145 automatable parameters:

- ID `0`: performance pitch, discrete `-24..24` semitones
- IDs `1..144`: 16 performance slots × 9 parameters
- each slot: momentary trigger, effect type, seven normalized macros

For zero-based slot `p`:

```text
trigger = 1 + p * 9
type    = trigger + 1
macro n = trigger + 2 + n, n in 0..6
```

Effect type values are:

| Value | Effect |
| ---: | --- |
| 0 | Off |
| 1 | Beat Repeat |
| 2 | Reverse |
| 3 | Tape Stop |
| 4 | Gate |
| 5 | Pitch Down |
| 6 | Pitch Reset |
| 7 | Pitch Up |
| 8 | Low Band |
| 9 | Mid Band |
| 10 | High Band |
| 11 | LoFi |

Macro parameters remain normalized for the host. The UI formats them
semantically as grid divisions, percentages, semitones, decibels, milliseconds,
hertz, bits, or effect-specific choices.

Slots are independent. A kit or host state may configure repeated instances of
the same exact effect type, and every continuous instance has its own runtime
processor state.

## Performance input and held state

Direct computer keys, MIDI, automatable trigger parameters, and pointer presses
are independent hold sources. Their union is the aggregate held state for each
slot. Releasing one source must not release the slot while another source still
holds it. Pitch actions occur once per aggregate released-to-held transition.

Hosts disagree about octave labels, so product documentation uses MIDI note
numbers:

| Note | Slot/action |
| ---: | --- |
| 60 | Slot 1: stutter |
| 61 | Slot 2: half-beat repeat |
| 62 | Slot 3: one-beat repeat |
| 63 | Slot 4: buzz |
| 64 | Slot 5: cell from two beats back |
| 65 | Slot 6: cell from four beats back |
| 66 | Slot 7: reverse |
| 67 | Slot 8: tape stop |
| 68 | Slot 9: gate |
| 69 | Slot 10: pitch down |
| 70 | Slot 11: pitch reset |
| 71 | Slot 12: pitch up |
| 72 | Slot 13: low band |
| 73 | Slot 14: mid band |
| 74 | Slot 15: high band |
| 75 | Slot 16: LoFi |

Pitch actions also accept `57..59` and `81..83`. MIDI held state is independent
for all 16 MIDI channels before channel masks are aggregated.

The direct computer-key layout follows physical positions, not layout-produced
characters:

| Slots | Physical key positions |
| --- | --- |
| 1–4 | `1` `2` `3` `4` |
| 5–8 | `Q` `W` `E` `R` |
| 9–12 | `A` `S` `D` `F` |
| 13–16 | `Z` `X` `C` `V` |

The editor control is labeled **DIRECT KEYS**. It is runtime editor state, not
one of the 145 host parameters. It is disabled by default for CLAP and VST3
instances and enabled by default in standalone. Disabling it, or pressing an
unmapped key, must leave host/DAW keyboard handling available. MIDI, automation,
and pointer operation remain supported in every target.

## Event timing

MIDI, parameter, direct-key, and transport changes apply at process-block
boundaries. Sample-accurate event segmentation is deferred. Code and tests must
not imply sample-accurate effect starts until that architecture is implemented.

When several new continuous requests are observed in one block, admission is
deterministic. Admission recency affects cap overflow and restoration only; the
audible chain order is always ascending slot number.

## Serial processing and admission

Continuous effects are Beat Repeat, Reverse, Tape Stop, Gate, Low Band, Mid Band,
High Band, and LoFi. Off and the three pitch actions are not continuous stages.
Pitch actions neither enter the audio chain nor consume admission capacity.

Every admitted continuous slot is a distinct serial stage, including repeated
instances of the same exact effect type. Active stages process in ascending slot
order, independent of press, suspension, or restoration order. Each stage's dry
reference is the signal entering that stage, and its output becomes the next
stage's input.

At most six continuous processors are active. Held, active, and suspended are
distinct states:

- a held active slot is audible;
- a held suspended slot retains its request but is excluded from processing;
- a released slot is neither active nor suspended.

A new continuous request at capacity suspends the oldest admitted active slot
and becomes active immediately. When capacity returns, still-held suspended
slots restore most-recently-held first. Restored stages resume their frozen
processor state and then process at their ascending slot position.

Suspension freezes mutable processor state. It does not freeze configured buffer
history: the slot's history continues recording and advancing at its chain
position while the processor is suspended.

## Buffer history

DSP reset prepares one independent stereo `f32` history for every slot, each
covering at least eight seconds per channel at the active sample rate. The serial
signal, effect calculations, and wrapper audio remain `f64`; conversion occurs
only at the history read/write boundary. No history allocation or resizing may
occur in `process`.

Every slot currently configured as Beat Repeat, Reverse, or Tape Stop records
the signal arriving at its own serial position before its processor is applied.
Recording continues while the slot is unheld, active, or suspended. This ensures
that a new press, release/repress, or restored suspended processor has exact
current lookback for the signal that stage would receive.

Changing one buffer effect type to another preserves the slot's history but
resets capture/playback processor state. Changing from a non-buffer type to a
buffer type clears the slot's history and starts cold. Changing effect type never
allocates in `process`.

## Runtime lifecycle

| Event | Processor state | Buffer history | Held/admission state |
| --- | --- | --- | --- |
| suspension at the cap | freeze | configured buffer keeps recording and advancing | remains held and suspended |
| restoration from suspension | resume frozen state | continuous, current lookback | active again |
| aggregate release | reset that slot's processor | configured buffer keeps recording | remove from active/suspended admission |
| next press after release | start reset processor | use continuously recorded history | request admission |
| buffer-to-buffer type change | reset for new type | preserve | retain or resolve admission from aggregate hold |
| non-buffer-to-buffer type change | reset for new type | clear and start cold | retain or resolve admission from aggregate hold |
| plugin/DSP activation or reset | reset all processors | clear all rolling and slot histories | clear all held/admission state |
| kit application/reset | reset all processors | clear all rolling and slot histories | release every input source and clear admission |
| host state restoration | reset all processors | clear all rolling and slot histories | release every input source and clear admission |
| transport start/stop/seek/tempo/position change | preserve | preserve | preserve |

Activation means plugin or DSP activation/reset, not admission of a held slot.
Restoring a suspended slot therefore does not clear history or restart its
processor.

## Real-time safety

`process` and its callees must perform no allocation, resizing, locking, logging,
file access, blocking, or unbounded work. All wrapper scratch, the rolling
history, and all 16 slot histories are allocated during `reset`.

Audio output must be finite. Wrapper output flushes samples below the `f32`
normal range to zero.

## DSP regression corpora

`contract/` contains two explicitly separated corpora:

- root `contract/*.budsp` and root `SHA256SUMS` are frozen
  `dsp-contract-v1` seed evidence from the experimental C++ engine;
- `contract/v2/` is the canonical `dsp-contract-v2-serial` corpus for the
  approved serial architecture.

V1 is never rewritten to make intentional serial changes pass. V2 retains the
useful v1 input scenarios with outputs captured from the canonical Rust serial
engine and adds fixtures for duplicate same-type stages, six-stage
suspend/restore, buffer history across suspension/release, and chain-position
lookback. Both versions require finite output and use `1e-7`
absolute-plus-relative `f64` tolerance. Each version has its own manifest and
checksum list.

The C++ provenance of v1 is evidence, not a promise that Buffer Uppercut loads
old binaries, state, presets, or identifiers. Future intentional sound changes
must create another version or explicitly extend an unreleased version through a
reviewed code, test, documentation, and ADR change. Never silently rewrite a
frozen expected-output file.

## Native kit format

`.bupreset` remains version 1. It contains:

1. eight-byte `BUPRESET` magic;
2. little-endian `u32` version, slot count, macro count, and UTF-8 name length;
3. at most 63 bytes of UTF-8 kit name;
4. little-endian `f64` performance pitch;
5. for each of 16 slots, a `u32` effect type and seven little-endian `f64`
   normalized macros.

The decoder rejects unsupported versions/layouts, invalid UTF-8, non-finite or
out-of-range values, truncation, and trailing bytes. Files are capped at 4096
bytes. The format already represents independent repeated same-type slots, so
the serial architecture does not require a kit schema version change.

Portable kits do not preserve momentary trigger or input-source held state. Kit
application resets performance pitch to zero, selects slot 1, releases all held
sources, resets admission and processor state, and clears every DSP history.

## Editor behavior

- Default size: `1120×700`; minimum size: `920×620`.
- MIDI, direct keys, automation, and pointer input illuminate the aggregate held
  slot state.
- Auto-select MIDI is editor-session state, defaults on, and selects a slot on
  MIDI press.
- Pointer press selects and activates a slot; release ends only the pointer's
  contribution to its aggregate hold.
- Direct keys use the physical four-by-four layout and honor the target-specific
  default policy above.
- Pads remain the primary visual surface. Rolling waveform history is secondary.
- When several held buffer slots qualify for captured visualization, the
  editor-selected held slot wins even if suspended; otherwise the
  lowest-numbered active buffer slot wins. If neither exists, rolling history is
  shown.
- A suspended buffer's capture/playhead processor state remains frozen in the
  visualization while its configured history continues recording.
- Complete plugin presets are loaded and saved through the host-native preset UI
  exposed by TRUCE wrappers. The editor does not duplicate the DAW's preset
  browser with LOAD/SAVE buttons.
- Continuous controls emit begin/set/end automation gestures.
- Unused macros are disabled.
