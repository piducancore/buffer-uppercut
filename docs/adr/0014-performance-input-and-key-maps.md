# ADR 0014: Host Trigger gestures and durable key maps

- Status: accepted design; host acceptance pending
- Date: 2026-09-24

## Context

Direct computer keys and pointer pads previously used private transient hold
channels. Although each pad already exposed an automatable Trigger parameter,
playing the editor did not report those gestures to the host. Fixed physical
keys also prevented user layouts. Adding mapping to only one persistence path
would make host presets and native kits recall different performance setups.

## Decision

Use each existing slot Trigger parameter for local key/pointer performance.
Combine local ownership into one balanced begin/1/0/end gesture per slot. Audio
reads the host parameter, without an additional private local audio hold. MIDI
remains an independent hold source. Host automation modes govern recording and
live override; no plugin recording switch is added. Effect types and macros
retain their existing automation paths and stable IDs.

Keep backend physical-code conversion and gesture routing in the wrapper.
`kit/src/keys.rs` owns a pure validated map with stable product identifiers,
unassigned slots, and unique nonzero bindings. Default physical positions remain
`1234/QWER/ASDF/ZXCV`. Learning does not play a pad; occupied assignments require
an explicit swap. Mapping changes release local gestures and require fresh
presses. Focus loss, close, disable, and recall also end local ownership.

Store the map as host persisted metadata and in native `.bupreset` version 3.
Host and native recall restore the same durable sounds, name, and map. Factory
kits restore their authored map too. Native v1 and v2 are deliberately rejected;
there is no migration requirement for these unreleased formats. Stable wire IDs
and exact format layout are specified in `CONTRACTS.md`.

Suppress existing high Trigger values during activation and host
restore, even if saved while held and recalled with the editor closed. Preserve
the raw host parameter values for deterministic host state round-trips. A fresh
Trigger automation event or local press clears suppression for that slot;
observing a low value also rearms it. MIDI remains independent. Clear
Active Shift and other runtime state as before; subsequent timeline automation
still applies. Direct Keys enablement remains runtime policy in both paths.

New host snapshots include the fixed `BUSTATE` + byte `1` custom-state marker.
TRUCE only invokes its state-changed hook when a custom payload is present;
the marker ensures released recall even with the editor closed. It is published
at construction (including save-before-first-process) and through the
preallocated snapshot API, not allocated in processing. Earlier
markerless host states are outside this recall guarantee; no migration is added.

Add narrow generic state-dirty notification to the TRUCE editor bridge and
CLAP/VST3 wrappers so metadata changes notify the host without unrelated
parameter gestures. Keep product mappings outside framework code.

## Consequences and limits

- Parameter count remains 144 and ID 0 stays unused.
- Recorded automation addresses slot identity, so later key remapping does not
  rewrite recorded gestures.
- Host read/touch/latch behavior can affect live parameter edits. Local sound
  and host playback are not independent ownership channels for one Trigger.
- Block-boundary processing can collapse very short taps. Lossless rapid-tap
  capture and real DAW recording/replay remain unaccepted until verified.
- Codec support does not provide a native-file browser. The editor continues
  relying on host preset UI for complete plugin preset loading/saving.
- This supersedes the private local audio-hold and fixed-map portions of ADR
  0004 and the native version policy of ADR 0013; the Pitch gesture and transient
  Active Shift decision remain unchanged.

## Rejected alternatives

- Zero host Trigger parameter values on recall: CLAP Validator's basic, binary,
  and buffered reproducibility checks demonstrate that this breaks the host's
  parameter-state contract. Suppress recalled holds instead of altering state.

- Mirror private local audio holds into host Triggers: delayed echoes and shared
  playback ownership complicate release semantics and can strand a held value.
- Add automatable mapping parameters: key assignments are durable configuration,
  while recorded performances should continue addressing fixed slot IDs.
- Preserve custom maps only in host presets: this creates different durable
  behavior from native presets and factory configurations.
- Fake an unrelated parameter edit to mark metadata dirty: this creates noise
  in automation and does not express the actual configuration change.
