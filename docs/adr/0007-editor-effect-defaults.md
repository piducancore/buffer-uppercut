# ADR 0007: Editor type changes load effect defaults

- Status: accepted
- Date: 2026-09-09

## Context

The seven host macro parameters are reused for different meanings by each effect.
Keeping their values when a performer selects another effect can move a zero
Jitter or Lookback value into Wet, leaving an active pad inaudible. Host startup
defaults must also agree with the current Classic kit rather than older macro
layouts.

## Decision

The editor type knob loads all seven values from the selected effect's DSP
defaults whenever it crosses into a different discrete type. It pins the edited
pad for the gesture and brackets type and macro writes with host begin/end edits.
These are individual host parameter writes, not an atomic host transaction.
The control is visually distinct and identifies its reset behavior.

Host type automation and complete preset/kit recall remain independent parameter
writes; they do not implicitly replace saved macro settings. Fresh host defaults
match the Classic kit across all slots and controls.

Knobs emit edit requests without assigning their incoming value property. The
parameter snapshot remains the source of dial position, text, and the next drag's
starting value when selecting another pad or restoring parameters.

## Consequences

- Manual type changes discard that slot's previous macro settings.
- Type automation recorded from the editor includes the accompanying macro edits.
- Restoring saved settings retains their exact values.
- Headless pointer-event tests cover knob synchronization after prior edits;
  parameter tests cover fresh host defaults against the canonical kit.

## Rejected alternatives

- Keeping unrelated macro values across manual type changes leaves valid effects
  unexpectedly dry or configured with unintended pitch and timing.
- Resetting macros in the DSP on every type change would overwrite host recall
  and independent automation.
- Caching dial values in the visual component would let the display and drag
  origin drift from the parameter store.
