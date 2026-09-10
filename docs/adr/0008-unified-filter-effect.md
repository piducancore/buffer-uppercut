# 0008 — Unify the performance filters behind a Mode control

## Status

Superseded in part by [ADR 0009](0009-held-grain-pitch.md), which renumbers
Filter, LoFi, and Vinyl while retaining this Filter design.

## Context

Low Band, Mid Band, and High Band were three effect choices backed by the same
state-variable filter. Their macro layouts differed, which made type changes
replace the meaning and position of several controls. The product needs the
three familiar filter responses without presenting them as unrelated effects.

## Decision

Replace effect values 8–10 with one Filter at value 8. Renumber LoFi to 9 and
Vinyl to 10. Keep all 145 host parameter IDs and the kit v1 binary layout, but
do not migrate unreleased state or automation that used the old effect values.

Filter always exposes Mode, Frequency, Resonance, Drive, Envelope, Motion, and
Wet. Mode has Low-pass, Band-pass, and High-pass positions. All modes use the
same nonlinear state-variable topology and derive bounded feedback from
Resonance.

Keep Classic pads 13–15 as distinct factory configurations: Low-pass at 260 Hz,
Band-pass at 1.2 kHz, and High-pass at 3.6 kHz. Pad labels show LP, BP, and HP,
while the effect selector shows Filter. Manual selection loads the generic
Band-pass configuration at 1.2 kHz.

Create `dsp-contract-v3-unified-filter` from the frozen v2 inputs and capture
the intentional output changes from the canonical Rust engine. Keep v1 and v2
unchanged.

## Consequences

- Moving between filter responses changes one discrete macro and preserves a
  stable meaning for the other six controls.
- At this decision point the selector had 11 choices and normalized type
  automation used index / 10. ADR 0009 defines the current numbering.
- Existing unreleased normalized type automation and `.bupreset` files using
  the former values 9–12 are intentionally incompatible.
- Low-pass and High-pass no longer expose a separate Feedback macro; their
  feedback follows Resonance like Band-pass.
- The v3 corpus preserves this milestone; ADR 0009 defines the current v4 sound
  oracle.

## Rejected alternatives

- Three band-pass frequency presets would remove the low-pass and high-pass
  spectral shapes.
- Keeping three effect choices would retain the misleading selector model and
  incompatible macro positions.
- Adding an eighth Filter macro would change the 145-parameter schema.
