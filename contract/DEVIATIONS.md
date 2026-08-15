# DSP contract deviations

## V1 to v2: capped serial performance slots

- Status: approved by ADR 0004
- Affected v1 scenarios: any scenario whose configured or held slots expose the
  previous newest-per-family topology, shared mutable effect state, shared
  buffer history, or `f64` history storage
- Adopted by: `dsp-contract-v2-serial`

The canonical engine now treats every held continuous slot as an independent
serial stage, including repeated instances of the same exact effect type. Active
stages run in ascending slot order with a six-processor admission cap. A fresh
request suspends the oldest admitted active processor; capacity restores
still-held suspended requests most-recently-held first.

Each of the 16 slots has independent processor state and an independently
prepared stereo `f32` history covering at least eight seconds per channel.
Configured buffer history captures that slot's exact chain-position input while
released, active, or suspended. Suspension freezes processor state but history
continues recording. Aggregate release resets processor state but history
continues while the slot remains buffer-configured.

These changes deliberately alter outputs formerly governed by
`dsp-contract-v1`, notably layered/buffer priority cases and any history read that
crosses the new `f32` storage boundary. V1 expected files and checksums remain
unchanged. V2 expected results were established from the approved canonical Rust
implementation, checked against focused unit tests for serial order, admission,
wet/dry locality, lifecycle, history position, reset, and determinism, and then
pinned by a separate manifest and checksum list.

V2 also adds explicit scenarios for duplicate gates, six-stage
suspend/restore, buffer suspension/release history, and buffer chain-position
lookback. The DSP, kit v1 schema, wrapper integration, authoritative
documentation, and ADR agree on the new behavior.

## Policy

Never rewrite a frozen versioned fixture silently. A later deliberate sound
change must record:

- the affected fixture and behavior;
- the motivation and audible consequence;
- how the expected result was established;
- the contract version that adopts the change; and
- confirmation that DSP, wrapper, kit/state, tests, and documentation agree.
