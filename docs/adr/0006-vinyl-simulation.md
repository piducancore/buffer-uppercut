# 0006 — Vinyl is an independent record-wear effect

## Status

Superseded in part by [ADR 0008](0008-unified-filter-effect.md).

## Context

LoFi provides rate and bit reduction. A held-pad Vinyl effect adds slow and fast
pitch instability, worn tone, saturation, and independently adjustable surface
textures within the existing seven-macro serial slot model.

## Decision

Append Vinyl as effect value 12. Keep the 16 slots, 145 parameter IDs, MIDI map,
and kit v1 binary layout. The type parameter now has 13 choices, so normalized
automation uses index / 12. Existing plain effect values remain unchanged; no
migration of old unreleased normalized state or automation is introduced.
The kit decoder accepts values 0–12 and continues rejecting unknown values.

Expose Wow, Flutter, Wear, Drive, Dust, Noise, and Wet. Their mappings and default
values are defined once in [CONTRACTS.md](../CONTRACTS.md#vinyl-macros).
Add Vinyl Cuts as a fifth factory kit; existing four kits are unchanged.

Use an original causal, linearly interpolated short delay for stereo-linked
pitch modulation, blended saturation and low-pass wear, and bounded procedural
surface texture. Noise and dust have independent level controls and are silent
at zero. Deterministic random seeds differ by slot. This is a creative simulation,
not a circuit model of a particular turntable or a physical pressing process.

`dsp/src/vinyl.rs` owns the algorithm, with lifecycle integration and metadata in
`dsp/src/lib.rs`. Allocate a 14 ms stereo f64 delay plus four samples for each
slot at activation. No allocation occurs on type change or trigger. Reset
invalidates the delay with a valid-sample counter; it does not clear the vector.
Suspension freezes all processor state, including the short delay. The existing
eight-second capture histories remain exclusive to Repeat, Reverse, and Stop.

## Consequences

- Delayed wet signal intentionally mixes with undelayed input and can produce
  coloration at partial Wet. Modulated delay is not reported as host latency.
- Startup limits delay to valid history, preventing silence or stale audio.
- Surface texture is audible on silence while held and active.
- Control smoothing handles automation; clean endpoints become exact when it
  settles. Zero Dust/Noise immediately removes their generated contributions.
- Storage costs approximately 16 × (ceil(sample_rate × 0.014) + 4) × 16 bytes:
  about 169 KiB at 48 kHz and 673 KiB at 192 kHz across all sixteen slots.
- Existing v1/v2 golden files are unchanged. New behavior is tested separately
  with signal properties and serial/lifecycle integration tests.
- Host listening, recall, automation, and worst-case CPU acceptance remain
  explicit release checks in TESTING.md and ROADMAP.md.

## Rejected alternatives

- Folding Vinyl into LoFi would displace its existing rate/bit controls.
- Reusing capture history would couple modulation to different suspension and
  recording semantics and use unnecessary eight-second storage operations.
- Additional host parameters or a new kit layout are unnecessary.
- Sampled crackle files would add assets and file loading where deterministic
  procedural synthesis is sufficient.
- A physical record/turntable circuit model is outside this performance effect's
  scope; oversampling can be considered after listening and CPU measurements.
