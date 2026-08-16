# 0005 — Performance bands use a bounded nonlinear resonant filter

## Status

Accepted.

## Context

The original Low Band, Mid Band, and High Band processors were first-order filters with only cutoff edges and wet mix. They were useful for clean isolation but could not provide resonance, drive, envelope response, motion, or controlled feedback. Buffer Uppercut needs a more expressive held-pad filter that remains deterministic, allocation-free, and safe in the capped serial chain.

The product is unreleased, so its effect semantics may change deliberately when code, tests, contracts, and documentation change together. The 145 host parameter IDs and seven normalized macro values per slot remain valuable and do not need replacement.

## Decision

Low Band, Mid Band, and High Band retain effect type values 8, 9, and 10. Each becomes a nonlinear topology-preserving state-variable filter with bounded feedback and output saturation.

Low and High expose `CUTOFF`, `RESONANCE`, `DRIVE`, `ENVELOPE`, `MOTION`, `FEEDBACK`, and `WET`. Mid exposes `CENTER`, `WIDTH`, `RESONANCE`, `DRIVE`, `ENVELOPE`, `MOTION`, and `WET`.

Frequency is logarithmic from 20 Hz to 20 kHz and is clamped below Nyquist at runtime. Resonance increases nonlinearly. Drive reaches 30 dB before the filter. A deterministic sine oscillator provides motion. A peak envelope follower raises cutoff with a fast attack and slower release. Feedback is saturated before reinjection. Mid derives its feedback character from resonance because all seven macros are otherwise assigned.

All mutable filter, envelope, oscillator, and feedback state is per slot. Release resets it, suspension freezes it, and activation/reset clears it under the existing serial-slot lifecycle.

## Consequences

- Existing parameter IDs, effect values, host automation lanes, and kit format remain unchanged.
- The meanings of band macro values change deliberately and old unreleased presets are not compatibility targets.
- The filters can self-emphasize and distort strongly but remain numerically bounded.
- Serial duplicates retain independent state and can produce substantially stronger nonlinear results.
- The canonical serial DSP corpus changes for scenarios that process a band stage.
- No allocation, locking, logging, filesystem access, or unbounded work is added to processing.

## Rejected alternatives

- Keeping the first-order filters was rejected because it did not provide the requested expressive behavior.
- Adding a new effect type was rejected because it would expand the public effect schema when the existing band roles are the intended replacement point.
- Reproducing a Sherman Filterbank circuit or branding was rejected. This is an original performance-filter design, not a hardware model or compatibility target.
- Unbounded linear feedback was rejected because serial stacking and automation extremes could produce runaway or non-finite output.
- Oversampling was deferred until measured listening and CPU acceptance justify its activation-memory and processing cost.
