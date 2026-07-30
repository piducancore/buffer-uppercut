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

## Parameter schema

There are exactly 145 automatable parameters:

- ID `0`: performance pitch, discrete `-24..24` semitones
- IDs `1..144`: 16 pads × 9 parameters
- each pad: momentary trigger, effect type, seven normalized macros

For zero-based pad `p`:

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
semantically as grid divisions, percentages, semitones, decibels,
milliseconds, hertz, bits, or effect-specific choices.

## MIDI

Hosts disagree about octave labels, so product documentation uses note numbers.

| Note | Action |
| ---: | --- |
| 60 | Pad 1: stutter |
| 61 | Pad 2: half-beat repeat |
| 62 | Pad 3: one-beat repeat |
| 63 | Pad 4: buzz |
| 64 | Pad 5: cell from two beats back |
| 65 | Pad 6: cell from four beats back |
| 66 | Pad 7: reverse |
| 67 | Pad 8: tape stop |
| 68 | Pad 9: gate |
| 69 | Pad 10: pitch down |
| 70 | Pad 11: pitch reset |
| 71 | Pad 12: pitch up |
| 72 | Pad 13: low band |
| 73 | Pad 14: mid band |
| 74 | Pad 15: high band |
| 75 | Pad 16: LoFi |

Pitch actions also accept `57..59` and `81..83`. Held state is independent for
all 16 MIDI channels. A pad is held when its trigger parameter or any channel
holds its mapped note. Pitch actions occur once per aggregate
released-to-held transition.

## Event timing

MIDI, parameter, and transport changes apply at process-block boundaries.
Sample-accurate event segmentation is deferred. Code and tests must not imply
sample-accurate effect starts until that architecture is implemented.

## Real-time safety

`process` and its callees must perform no allocation, resizing, locking,
logging, file access, blocking, or unbounded work. All block scratch and DSP
history are allocated during `reset`.

Audio output must be finite. Wrapper output flushes samples below the `f32`
normal range to zero.

## DSP regression corpus

`contract/` is a frozen seed corpus captured during the experimental phase. It
continues to protect the current sound at `1e-7` absolute-plus-relative `f64`
tolerance. Its C++ provenance is evidence about expected samples, not a promise
that Buffer Uppercut will load old binaries, state, presets, or identifiers.

Future intentional sound changes create a new fixture version owned by this
repository. Never rewrite a frozen expected-output file silently.

## Native kit format

`.bupreset` is a product-owned binary format. Version 1 contains:

1. eight-byte `BUPRESET` magic;
2. little-endian `u32` version, pad count, macro count, and UTF-8 name length;
3. at most 63 bytes of UTF-8 kit name;
4. little-endian `f64` performance pitch;
5. for each of 16 pads, a `u32` effect type and seven little-endian `f64`
   normalized macros.

The decoder rejects unsupported versions/layouts, invalid UTF-8, non-finite or
out-of-range values, truncation, and trailing bytes. Files are capped at 4096
bytes.

Portable kits do not preserve momentary trigger state. The editor resets
performance pitch to zero when loading a kit, selects pad 1, releases all held
pads, and clears captured audio.

## Editor behavior

- Default size: `1120×700`; minimum size: `920×620`.
- MIDI notes illuminate pads.
- Auto-select MIDI is editor-session state, defaults on, and selects a pad on
  MIDI press.
- Pointer press selects and activates a pad; release ends the trigger.
- Continuous controls emit begin/set/end automation gestures.
- Unused macros are disabled.
- Unhandled keyboard events pass back to the host so REAPER's virtual MIDI
  keyboard remains usable while the editor is focused.
