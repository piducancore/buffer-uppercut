# ADR 0016: Visualize the complete pad-triggered performance chain

- Status: accepted
- Date: 2026-10-23

## Context

The selected captured waveform hid other simultaneous processors. The rolling
waveform showed input, leaving non-buffer effects visually unexplained. Pads
already form a deterministic, press-ordered serial chain with six active stages.

## Decision

The main stereo timeline always shows final output over a muted input reference,
covering the same newest four beats (limited by prepared history). Selecting a
pad never changes this timeline. Inside the same panel, active stages appear in
press order, labeled by slot, physical key, and effect. Clicking a stage
selects its pad without triggering it. Suspended stages remain in a separate
orange row, outside the audible chain. Pitch action pads do not become stages.

Each buffer stage has an independent sampled capture trace, read position,
direction, playback rate, and duration. Other stages have schematic glyphs:
gate windows, dual grain windows, filter response, quantization steps, and Vinyl
modulation. Vinyl dust has an 80 ms display envelope driven by actual impulses;
its surface-noise marking remains schematic. These glyphs describe processor state rather than claiming to be
measured spectra or intermediate audio waveforms. Wet/depth controls their
emphasis; zero wet is explicitly labeled DRY.

Extend the sequence-checked, 30 Hz atomic snapshot with aligned input/output bins
and sixteen fixed stage records. Capture traces use 32 evenly spaced samples
per stage, with bounded extraction work. Path construction remains on the UI
thread. One additional stereo f32 output history and fixed 32-sample peak caches are
allocated during reset;
processing writes it at the input history's corresponding position. No host
parameters, preset fields, audio algorithms, or admission rules change.

The editor no longer uses ADR 0004's selected-buffer visualization priority.
The DSP's focused capture helper remains available for diagnostics and tests.

## Consequences

- Every audible stage remains identifiable, including repeated effect types.
- Different captures never share a misleading common cursor time axis.
- Output history adds two f32 channels at the prepared eight-second ring size.
- Snapshot readers retain the entire previous frame if publication overlaps.
- Complete 32-sample chunks use cached peaks; partial boundaries read raw samples.
  Tests require bit-identical bins to raw scans across wrap and partial chunks.
  Host CPU acceptance must include visualization publishing.
- Small stage traces are sampled illustrations, not peak-preserving waveforms.

## Rejected alternatives

- Last-triggered or selected effect replacing the main waveform: hides the mix.
- All cursors on one capture: implies incompatible captures share a time scale.
- A separate static signal-route strip: duplicates the live chain.
- UI-only clocks for processor motion: continue moving while DSP is suspended.
