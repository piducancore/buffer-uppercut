# Recordable performance and configurable keys

Status: implemented and installed; DAW acceptance pending.
Last updated: 2026-09-24.

## Implementation status

The implementation uses host-authoritative Trigger values for combined local
key/pointer gestures (`src/input.rs`), with independent MIDI holds. Physical-key
conversion lives in `src/keyboard.rs`; stable validated identifiers and maps
live in `kit/src/keys.rs`. Learn, Clear, Reset Layout, and explicit conflict swaps
edit the map shared by labels and dispatch. Host metadata and native v3 presets
carry the same durable sounds, name, and map. Recall/activation suppress restored
holds without rewriting the host's Trigger values. Generic framework state-dirty notification supports
mapping-only edits. See [ADR 0014](adr/0014-performance-input-and-key-maps.md).

The initial real-host prototype gate was not completed before implementing the
broader design; it remains pending. The phases below retain the acceptance work
and rationale, not claims that the gates have passed. Automated checks, visual
baseline checks, builds, and installed CLAP validation pass; see the
[evidence and remaining host checklist](TESTING.md#performance-input-acceptance).
VST3 pluginval is unavailable and interactive REAPER access timed out.

Rapid press/release events within one process block can collapse to released,
including Pitch actions. Lossless rapid-tap recording is not claimed. Native
file import/export still has no embedded editor UI.

## Product objective and viability

A performer should be able to play pads with computer keys or the pointer,
record their pad gestures through the DAW's automation system, and replay the
performance. Changing a physical key assignment must not alter existing pad
automation. Preserve the held Pitch Trigger plus repeated Down/Up tap gesture.

Recording is high-value core behavior for a performance effect. Key remapping
is useful for ergonomics and keyboard preferences, but can ship independently.
The existing 16 Trigger parameters and TRUCE editor automation bridge make
recording plausible without new parameters or a DSP algorithm change. Actual
host recording and release behavior must be proven before implementing the
whole feature: a successful parameter setter call does not prove recording.

Read AGENTS.md and its required documents first. Preserve the existing working
tree changes, including ADR 0013, the 144-parameter schema, and kit v3. Parameter
ID 0 stays unused; IDs 1..144 retain their meanings. Active Shift stays transient.

## User behavior

- Direct Keys remains opt-in in plugins; pointer pads always work.
- Enabled direct keys and pointer presses report ordinary pad Trigger gestures
  to the host. Prefer the host's automation recording controls over adding a
  plugin-specific recording switch. Document the host's required setup.
- Record slot Trigger values: 1 for held, 0 for released. MIDI remains a
  separate input and is not automatically copied into automation.
- One physical key per slot, with an unassigned option. Retain the current
  physical-position default. Support a validated initial set of letters,
  digits, and punctuation; modifier chords and system keys are out of scope.
- A mapping editor offers Learn, Clear, Reset Layout, and explicit conflict
  handling. An occupied assignment offers a swap; never silently displaces it.
- Labels and dispatch use the same mapping data. Slot identity and serial
  processing order stay fixed when keys change.
- Factory and user presets carry the map. Host state and native preset recall
  restore the same durable configuration. Factory recall therefore restores
  its authored mapping too. No effects-only recall mode in this increment.
- Direct Keys enablement remains the existing runtime policy in both recall
  paths. Never persist held inputs, Active Shift, or in-progress gestures.

These defaults are recorded in ADR 0014. Host behavior must still pass the
acceptance gates below.

## Phase 1: prove recording with the existing layout

Owner: input/host integration agent. Do this before mapping or codec work.

Inspect src/editor.rs, src/params.rs, src/lib.rs, and the installed TRUCE 6.3
CLAP/VST3 editor bridges. Determine where UI edits update the parameter store,
when hosts echo changes, what thread owns callbacks, and how gestures survive
editor teardown. The physical-key callback currently captures Params rather
than PluginContext and is Send + Sync; do not assume Slint Rc state can cross
that boundary. Read PATCH.md before any vendor change. Prefer product-layer
integration using existing generic APIs.

Prototype one pad with the fixed layout. The candidate gesture is begin_edit,
set 1 on initial press, then set 0 and end_edit on final local release. Combine
key and pointer ownership of that pad so overlapping local sources produce one
balanced gesture. Avoid frame-timer polling of held masks: it can lose fast taps.

Do not treat the previous proposal to mirror private holds into the Trigger
parameter as a proven design. Mirrored values and automation playback share
the same parameter; a delayed echo of 1 can outlive the private hold, and a
local 0 can overwrite playback. Trace and test this explicitly. Choose either
a host-authoritative trigger path or a bounded local override with a documented
handoff, based on the actual wrapper behavior. Do not claim complete independence
of live edits and automation on the same parameter. MIDI must remain independently
held. If a narrow framework change is necessary, document its generic boundary.

Prototype acceptance:

- Ableton Live VST3: record into Arrangement with Automation Arm, inspect both
  edges, replay without live input, then save/reopen and replay again.
- Test Session automation separately and record required Live settings.
- REAPER VST3 and CLAP: record/playback using appropriate write/touch modes;
  also exercise read mode and playback override behavior.
- Use held notes, rapid taps, overlapping key/pointer holds, and focus loss.
- Stop recording while held; release after stopping; restart playback. Verify
  no stuck pad or unmatched gesture. Document host latch semantics accurately.

Evidence must identify host/version, OS, format, settings, build, recorded
envelope, and observed playback. If a required host is unavailable, mark that
acceptance pending; another host's success is not a substitute.

## Phase 2: bounded pad gesture routing

Owner: same integration agent, after Phase 1 resolves ownership.

Introduce a small product-layer dispatcher for key and pointer pad transitions,
with explicit source ownership, balanced host gestures, and cleanup. Route host
playback and MIDI through their existing appropriate paths; never feed playback
back into outgoing automation. Preserve aggregate edge semantics for Pitch.

Cleanup covers key repeat, focus loss, editor close/reopen, Direct Keys disable,
mapping replacement, kit application, host-state restore, and activation.
Releases refer to the pad captured at key-down. Invalidate old gestures across
recall so a late release cannot clear a new gesture. Mapping edits release local
holds and require a fresh press; suppress repeats from keys still physically down.

Preserve block-boundary timing. A press and release within one block can be
lost by the current final-mask model; measure this and either implement bounded
ordered edge handling with tests or state a tested limitation and defer release
of the affected recording claim. Do not quietly introduce sample-accurate DSP
segmentation. Never use an unbounded queue, audio-thread lock, or allocation.

Test multiple Pitch Triggers, successive Down/Up taps, MIDI overlapping local
holds, automation echo ordering, and teardown. Replaying recorded slot gestures
must reconstruct Active Shift without restoring a global pitch parameter.

## Phase 3: mapping and unified persistence

Owner: configuration/codec agent, after dispatcher interfaces are settled.

Move physical mapping responsibility out of src/midi.rs into an appropriate
keyboard/input module. Store stable, validated key identifiers, not Rust enum
discriminants or platform scancodes. Keep keyboard backend and host types out
of dsp and kit. A small pure mapping representation may live alongside durable
kit configuration, with native conversion in the integration layer.

Define one durable configuration model shared by native kit encode/decode and
host-state adapters. Store mappings in host persisted metadata rather than
automatable parameters. Validate the complete map before publishing it; reject
duplicates and unsupported identifiers without applying partial configuration.
Ensure configuration-only edits mark host state dirty using a supported path.

Mappings use native version 3, which rejects versions 1 and 2. Confirm that
policy and malformed-map rejection without partial publication. No archived-port
migration is introduced.

Audit host serialization of momentary Trigger parameters: private held bits are
ephemeral today, but host Trigger values may be serialized. Suppress restored
high Trigger values through a supported wrapper lifecycle path so host/native
preset recall begins sonically released without changing the host's restored
parameter snapshot. A fresh host event or local press clears suppression.
Preserve subsequent timeline automation playback. Test this with the editor
closed as well as open. Avoid filesystem/UI calls from the audio callback.

Verify equivalent durable configuration and released runtime state after host
and native round-trips, including a save during a held performance. The native
codec currently has no embedded import/export UI: do not claim a user-facing
native preset workflow from codec tests alone or silently expand this task into
a preset browser. Record that remaining delivery scope explicitly.

## Phase 4: mapping UI

Owner: editor agent, after mapping and dispatcher contracts are settled.

Implement compact mapping controls and live labels in the native editor.
Learning captures assignment without playing a pad, supports cancel, suppresses
the assignment key until release, and respects focus/close cleanup. Explain
physical-key behavior clearly on non-US layouts. Preserve host passthrough for
disabled and unmapped keys. Check minimum/default/wide sizes and mapping states.

## Phase 5: integration, documentation, and installed verification

Owner: lead/integration agent.

Update CONTRACTS.md, ARCHITECTURE.md, TESTING.md, ROADMAP.md, kit documentation,
and a new ADR together. Mark the plan complete only after its acceptance gates
are satisfied; distinguish implemented behavior from unverified host behavior.
Keep AGENTS.md concise and update it only if ownership or commands change.

Run every AGENTS.md required command, all committed screenshot checks, relevant
validators, and corpus integrity checks. No sound-fixture regeneration is
expected unless an intentional sound/timing change is separately documented.
Install CLAP/VST3, verify installed executable hashes, and retest the actual
installed build in the target DAWs. Coordinate a DAW restart without discarding
an open user project. Do not equate a successful build with host acceptance.

For multi-agent execution, Phase 1 and dispatcher ownership decisions are the
critical path. After interfaces settle, codec and UI work may proceed in
separate file ownership areas; the lead integrates changes to src/editor.rs,
src/params.rs, and src/lib.rs to avoid competing edits. Each agent reports files,
tests, unresolved limitations, and evidence. No agents are dispatched by this plan.

## Host references

- [Ableton: recording and editing automation](https://www.ableton.com/en/manual/automation-and-editing-envelopes/)
- [Ableton: plug-in controls and automation](https://www.ableton.com/en/live-manual/12/using-plug-ins/)

Live controls whether exposed parameter edits are recorded. Track recording
alone must not be advertised as sufficient in every automation configuration.
