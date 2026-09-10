# 0009 — Pitch uses held grain processing with action roles

## Status

Accepted.

## Context

Pitch Down, Pitch Reset, and Pitch Up were separate edge-triggered effect types
that changed a global pitch value. Their behavior differed from held audio
effects, and varispeed pitch also changed buffer playback timing. The desired
performance gesture is to hold one pitch key, tap another key repeatedly to move
by a configured semitone step, and return to normal on release.

## Decision

Replace effect values 5–7 with one Pitch at value 5. Renumber Filter to 6,
LoFi to 7, and Vinyl to 8. Keep all 145 parameter IDs and the kit v1 binary
layout. Unreleased state and automation using the earlier values are not
migrated.

Pitch exposes Role, Step, Grain, Texture, Smooth, Feedback, and Wet. Role is a
three-position Down, Trigger, or Up choice. Only a held Trigger role enters the
serial audio chain. A newly pressed Down or Up role changes the active shift by
its Step once, only while at least one Trigger is held. Releasing the final
Trigger resets the shift to zero. Action roles never consume admission capacity.

Use an allocation-free dual-grain delay shifter. Grain ranges from 12–120 ms,
Smooth from 2–120 ms, Texture adds deterministic read-position movement, and
Feedback is bounded below unity. Each slot prepares an independent 140 ms stereo
`f64` delay during reset. Release, role change, type change, kit reset, and host
state restoration reset it in constant time by invalidating its stored samples.
Suspension freezes the grain processor.

Classic pads 10–12 become Pitch Down, Pitch Trigger, and Pitch Up configurations
on S, D, and F. The active accumulated shift remains visible through the existing
Performance Pitch parameter, labeled Active Shift in the editor. Beat Repeat
and Reverse rename their local Pitch macro to Slice Pitch and no longer add the
active global shift to buffer playback rate.

Create `dsp-contract-v4-grain-pitch` from the frozen v3 inputs and capture the
intentional sound and numbering changes from the canonical Rust engine.

## Consequences

- Repeated action taps accumulate between −24 and +24 semitones while Trigger
  remains held.
- Trigger release replaces the former Pitch Reset gesture.
- Pitch changes preserve input duration and host-tempo timing while adding an
  intentionally grainy, low-latency texture.
- The selector has nine choices and normalized type automation uses index / 8.
- The additional prepared memory is bounded and process-time work remains
  allocation-free.

## Rejected alternatives

- Varispeed would continue coupling pitch to duration.
- A transparent phase-vocoder design would add more latency and complexity than
  the intended performance texture.
- Keeping a Reset role would duplicate the release gesture.
- Letting action roles enter the chain would waste admission capacity and could
  suspend the audible Trigger.
