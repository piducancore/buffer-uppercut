# ADR 0003: Audio-to-UI data uses bounded atomic snapshots

- Status: accepted
- Date: 2026-07-30

## Context

The editor needs real DSP history and captured-slice information at interactive
frame rates. The audio thread cannot allocate, lock, log, access files, or wait
for the UI.

## Decision

The DSP fills 256 preallocated stereo min/max bins. The wrapper publishes them
at 30 Hz through a fixed sequence-checked atomic bridge stored outside the host
parameter schema. Slint reads a complete snapshot or retains its previous
frame, then creates path geometry on the UI thread.

## Consequences

- Publication is bounded and wait-free from the audio thread's perspective.
- Readers never render partially published frames.
- Memory and publication cost are fixed.
- Spectrum analysis or higher-resolution displays require a separately reviewed
  data path and performance budget.

## Rejected alternatives

- Mutex-protected waveform buffers: can block the audio callback.
- UI access to the DSP ring buffer: creates unsafe lifetime and concurrency
  coupling.
- Allocating/sending dynamic vectors from `process`: violates realtime rules.
