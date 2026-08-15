# ADR 0004: Performance slots form a capped serial keyboard chain

- Status: accepted
- Date: 2026-08-15
- Implementation: current canonical architecture

## Context

Buffer Uppercut previously selected at most one processor from each effect
family and shared mutable history and effect state across slots. That topology
prevented two slots of the same type from acting as independent serial stages.
The editor also treated computer-key events only as host input rather than as a
first-class, keyboard-shaped performance surface.

The serial performance-key proposal requires deterministic same-type stacking,
independent per-slot runtime state, bounded overload behavior, and direct keys
without removing MIDI, automation, or pointer control. Buffer stages make the
resource and lifecycle choices consequential: exact serial lookback requires
history captured at each stage's chain position, while sixteen long stereo
`f64` histories would have an excessive worst-case memory cost.

This record captures the owner-approved G1 product defaults implemented by the
canonical DSP, wrapper, editor, fixtures, and current-state documentation.

## Decision

### Slot model, schema, and serial order

Retain 16 performance slots and the existing 145 host parameter IDs. ID `0`
remains performance pitch, and IDs `1..144` remain 16 groups of trigger, effect
type, and seven macros. This change does not renumber parameters or add legacy
state migration.

Each slot owns independent effect runtime state and an independently prepared
buffer history. Input adapters keep keyboard, MIDI, automation, and pointer
holds separate, then aggregate them into one held state per slot. Releasing one
source cannot release a slot that another source still holds.

Every admitted continuous effect is a distinct stage, including repeated
instances of the same exact effect type. Active stages process in ascending slot
order, independent of press or admission order. A stage's wet/dry reference is
the signal entering that stage. Buffer stages consume and capture the serial
signal at their own chain positions. Pitch Down, Pitch Reset, and Pitch Up
remain edge-triggered actions, do not enter the audio chain, and do not consume
the continuous-processor cap.

### Direct computer keys

The sixteen slots map by physical key position in this four-by-four layout:

| Slots | Physical key positions |
| --- | --- |
| 1–4 | `1` `2` `3` `4` |
| 5–8 | `Q` `W` `E` `R` |
| 9–12 | `A` `S` `D` `F` |
| 13–16 | `Z` `X` `C` `V` |

The mapping follows those physical positions rather than the characters
produced by the active keyboard layout. Any framework work needed to expose
physical-key identity must remain a narrow generic capability, not product
behavior embedded in `vendor/truce-slint`.

Direct keys are opt-in for CLAP and VST3 plugin instances so the default does
not capture DAW shortcuts or duplicate a host virtual MIDI keyboard. Direct
keys are enabled by default in the standalone application. MIDI, automation,
and pointer operation remain supported in every target.

### Admission cap and overflow

At most six held continuous processors are admitted to the audible chain. Held,
active, and suspended are separate states.

When a newly held continuous slot exceeds the cap, admit the newest slot and
suspend the oldest admitted active slot. A suspended slot remains held but is
excluded from processing. When capacity returns, restore still-held suspended
slots most-recently-held first. Admission and restoration recency never alter
the ascending slot order of the audible chain.

Suspension freezes a slot's mutable processor state. For a buffer slot this
includes capture/playback phase and related processor state, but not its
configured history. The history continues recording and advancing with the
signal arriving at that slot's chain position. Restoration resumes the frozen
processor state against exact current lookback rather than resetting it.

### Buffer history and effect-type changes

Prepare one independent stereo `f32` history for every slot during DSP reset.
Each history must cover at least eight seconds per channel at the active sample
rate. No history allocation or resizing may occur in `process`.

The serial signal and effect calculations remain `f64`. Samples are converted
only when written to or read from the `f32` histories. A configured buffer stage
records the signal arriving at its own serial position whether it is released,
active, or suspended.

Changing a slot from one buffer effect to another buffer effect preserves that
slot's history. Changing a slot from a non-buffer effect to a buffer effect
clears that slot's history and begins cold. This rule makes automated
effect-type changes deterministic without allocating or fabricating unavailable
lookback.

### Visualization selection

When multiple buffer slots could supply the captured-slice visualization, use
the editor-selected slot first if it is a held buffer slot. This includes a
selected held slot that is suspended: its capture/playhead processor state is
frozen while its configured ring continues recording. If the selected slot does
not qualify, use the lowest-numbered active buffer slot. If neither exists, show
rolling history.

### Runtime lifecycle

Runtime state follows these rules:

- suspension freezes processor state and resumes it on restoration;
- aggregate release resets that slot's effect processor state, while its buffer
  history remains available and continues recording while configured;
- plugin/DSP activation, kit reset or application, and host state restoration
  clear every slot history and reset transient processor and admission state;
- transport start, stop, seek, tempo, and position changes preserve processor
  state, histories, and held/admission state.

Activation in this lifecycle means plugin or DSP activation/reset, not admission
of a held slot into the six-stage chain. Restoring a suspended slot therefore
does not clear its history.

## Consequences

- The host schema and all 145 IDs remain stable through the serial-chain change.
- Same-type effects become independently stateful and audibly stack in a
  deterministic order.
- Overload behavior is bounded and musically reversible: the newest request is
  heard immediately, while the most recent still-held suspended request returns
  first when capacity becomes available.
- Six active processors bound processing work, but the sixteen prepared
  histories still create a sample-rate-dependent activation memory cost that
  must be measured and accepted at supported rates.
- `f32` storage halves history memory relative to `f64` storage, with deliberate
  history quantization at the storage boundary while processing remains `f64`.
- A non-buffer-to-buffer automation change cannot look backward before the cold
  transition. Buffer-to-buffer changes retain lookback continuity.
- Plugin users must explicitly enable direct keys; standalone users receive the
  keyboard-first interaction without setup.
- Release starts the next trigger with reset processor state, while transport
  operations do not disrupt a performance.
- The frozen v1 corpus remains historical evidence. Intentional serial output
  changes are owned by the separately checksummed v2 serial corpus.

## Rejected alternatives

- Reducing the product to 12 slots: gives up four existing slots and changes the
  host and kit schemas without enough benefit for the first serial design.
- Renumbering or expanding the 145 parameters: unnecessary for the approved
  slot count and creates avoidable host-state churn.
- Logical-character mapping: moves the performance surface across keyboard
  layouts instead of preserving physical playing positions.
- Enabling direct keys by default in plugins: risks DAW shortcut capture and
  duplicate triggering through host virtual MIDI keyboards.
- Caps of four, eight, or unlimited processors: four is too restrictive, while
  eight or unlimited increases worst-case processing without the approved
  six-stage performance envelope.
- Ignoring the newest press at capacity: makes a fresh performance gesture
  inaudible. Permanently dropping the oldest slot also loses reversible held
  intent.
- Shared, pooled, or stereo `f64` histories: shared or pooled histories cannot
  guarantee independent serial capture, while per-slot `f64` storage doubles
  the approved history memory cost.
- Preserving history on non-buffer-to-buffer changes: exposes stale audio that
  was not captured for the newly selected buffer stage. Clearing every
  buffer-to-buffer change instead discards valid same-slot serial history.
- Choosing the newest active buffer for visualization: makes display selection
  depend on press timing rather than explicit editor selection and slot order.
- Advancing a suspended processor, freezing configured history during
  suspension or release, preserving processor state after release, or clearing
  state on transport changes: each breaks the approved processor-freeze,
  continuous-history, release-reset, and transport-preserving lifecycle.
