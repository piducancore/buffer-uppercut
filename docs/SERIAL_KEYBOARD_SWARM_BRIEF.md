# Serial performance-key architecture: JCode swarm brief

- Status: proposed implementation brief, not a current product contract
- Prepared: 2026-08-15
- Target repository: `piducancore/buffer-uppercut` (Rust/TRUCE canonical implementation)
- Target harness: JCode `swarm-deep`, researched against installed JCode v0.75.3

This document is intentionally separate from `CONTRACTS.md` and the accepted
ADRs. It describes a proposed product change. The current contracts remain in
force until the decision gates below are resolved and a new ADR, code, tests,
and authoritative documentation land together.

## 1. Coordinator mandate

Use JCode's DAG-first deep swarm for research, design, implementation, critique,
and verification. Do not treat this file as permission to invent unresolved
product behavior.

The coordinator must:

1. Read `AGENTS.md`, `README.md`, `docs/ARCHITECTURE.md`,
   `docs/CONTRACTS.md`, `docs/ROADMAP.md`, `docs/TESTING.md`, and the relevant
   ADRs before assigning implementation work.
2. Run `git status --short` and preserve all existing changes.
3. Seed an explore-first task DAG. Exploration artifacts must feed the design
   and implementation nodes; chat summaries are not substitutes for artifacts.
4. Rely on deep mode's server-inserted critique and verify gates. Do not model
   gates as ordinary seed nodes or invent unsupported terminal-action kinds.
5. Stop at Decision Gate G1 if the owner has not supplied the missing product
   decisions. Report evidence and recommendations; do not choose silently.
6. Keep archived C++ and WRAC repositories out of scope. They are not
   compatibility targets or implementation dependencies.
7. Preserve the repository's real-time rules: no allocation, resizing, locks,
   logging, filesystem access, blocking, or unbounded work in `process` or any
   function it calls.

Every worker handoff should contain:

- `findings`
- `evidence` (file and line, test, measurement, or commit reference)
- `edge_cases_considered`
- `validation`
- `open_questions`
- `confidence`
- `what_i_did_not_check`

This matches JCode's current typed handoff model. A completion report must also
state the outcome, changed files or findings, validation run, and blockers.

## 2. Product intent

Buffer Uppercut should feel like a computer-keyboard performance instrument,
while retaining MIDI, automation, and pointer control as compatibility inputs.
Its continuous effects should form one deterministic serial chain:

```text
host input
  -> active performance slot 1
  -> active performance slot 2
  -> ...
  -> active performance slot N
  -> host output
```

An active slot consumes the output of the preceding active slot. Effects of the
same type are not collapsed: two gates, filters, LoFi stages, repeats, reverses,
or tape stops must be two independently stateful serial processors.

The Sherman FilterBank is a character reference, not a circuit-emulation
specification. The relevant traits are performance immediacy, controls with a
clearly audible effect, aggressive nonlinear interaction, modulation, and
routing that lets stages change one another. Do not claim to model Sherman
hardware or copy its control set. Sherman's own documentation describes a
dual-filter path with serial/parallel routing, overdrive, ADSR/envelope
following, LFO, FM/AM, and tracking; Buffer Uppercut is borrowing the
interactive attitude, not that topology.

## 3. Decisions already made

Treat these as requirements unless the owner explicitly reopens them:

1. Continuous effects stack serially, including effects from the same family
   and effects of the same exact type.
2. Buffer effects also run serially. Do not mix separate buffer-effect outputs
   in parallel against a shared dry source.
3. Each continuous slot has independent runtime DSP state. Algorithm code is
   shared; this does **not** mean one plugin instance, object graph, or thread
   per slot.
4. A stage's wet/dry calculation is local to that stage: its “dry” reference is
   the signal entering that stage, not the original host input.
5. MIDI, host automation, and pointer operation remain supported even if the
   product becomes keyboard-first.
6. The serial order is deterministic and does not change merely because keys
   were pressed in a different order. The working proposal is ascending
   performance-slot order.
7. Pitch Down, Pitch Reset, and Pitch Up are edge-triggered actions, not
   continuous audio processors. They do not occupy an audio-chain position or
   consume an active-processor slot.
8. The project is unreleased, so its current schema may change, but only via a
   deliberate ADR plus coordinated code, tests, fixtures, and documentation.

## 4. Decisions that are deliberately open

These must be resolved at G1. The swarm may research, measure, prototype in
throwaway tests, and recommend; it may not bury a choice in production code.

| Decision | Required evidence | Candidate choices to compare |
| --- | --- | --- |
| Number of performance slots | keyboard playtest, UI fit, schema impact | retain 16 for the first serial prototype; reduce to a keyboard-shaped 12; another evidenced layout |
| Computer-key mapping | physical-position feasibility, keyboard-layout behavior, host conflicts | logical characters; physical scan-code positions if the framework can expose them; user-remappable mapping |
| Direct-keyboard policy | CLAP/VST3/standalone tests in real hosts | opt-in in plugins and on by default standalone; another explicit focus/passthrough policy |
| Active continuous-processor cap | CPU benchmarks and musical overlap tests | compare at least 4, 6, 8, and uncapped; choose a product constant or an exposed control deliberately |
| Overflow behavior at the cap | state-machine tests and playtest | newest press suspends oldest active and restores it on release; ignore newest press; another documented policy |
| Buffer-history policy | memory table, sound tests, worst-case DAW activation | exact per-slot `f64`; reduced history duration; `f32` history with `f64` processing; bounded history pool with explicitly degraded cold-start behavior |
| Effect-type automation into a buffer stage | no-allocation proof and lookback expectations | cold history after type change; pre-armed history; restrict or defer instant lookback |
| Multiple-buffer visualization | UI prototype and deterministic selection rule | selected slot; earliest/last active buffer; compact multi-stage display |
| Runtime-state lifecycle | regression tests | reset or preserve phase/history when suspended, released, reactivated, kit-changed, state-restored, or transport-changed |

If G1 is reached without owner answers, the required deliverable is a concise
decision memo with measured tradeoffs and a recommended coherent bundle. Stop
there.

## 5. Repository-grounded starting point

The swarm must verify these observations itself; they are pointers, not a
replacement for inspection.

### Current contracts

- `NUM_PADS` is 16 in both the DSP and parameter layers.
- The host schema is 145 parameters: performance pitch plus 16 groups of
  trigger, effect type, and seven macros.
- MIDI notes 60–75 map to the 16 pads. MIDI held state is represented by one
  `u16` mask per channel; all 16 channels are aggregated.
- `.bupreset` version 1 encodes exactly 16 pads and seven macros per pad.
- Keyboard events currently pass back to the host so REAPER's virtual MIDI
  keyboard can continue working while the editor is focused.
- Events take effect at block boundaries; sample-accurate segmentation is not
  part of this change unless separately approved.

### Current DSP topology

`dsp/src/lib.rs::Engine` currently owns one shared stereo history and one shared
state set for the buffer, gate, filter bands, and LoFi categories. It records
press order, selects the newest held slot in each category, then processes at
most one buffer, one low band, one mid band, one high band, one LoFi, and one
gate. This is why same-category pads replace one another instead of stacking.

The current order is effectively:

```text
input -> newest buffer -> selected bands -> newest LoFi -> newest gate -> output
```

That topology must be replaced, not wrapped with additional family selectors.

### Current buffer allocation

`Engine::reset` allocates one stereo `f64` ring sized to the next power of two
at or above 16 seconds. One such history costs approximately:

| Sample rate | Samples/channel after power-of-two sizing | One stereo `f64` history | 16 histories |
| ---: | ---: | ---: | ---: |
| 48 kHz | 1,048,576 | 16 MiB | 256 MiB |
| 96 kHz | 2,097,152 | 32 MiB | 512 MiB |
| 192 kHz | 4,194,304 | 64 MiB | 1 GiB |

The current Classic kit configures eight buffer effects. Tape Lab configures
twelve. A maximum **active** pad count does not by itself bound this history
memory.

## 6. Required serial semantics

### 6.1 State model

Keep configuration and runtime state distinct:

```text
PerformanceSlot
  config: effect type + normalized macros (host/preset state)
  input state: independent keyboard, MIDI, automation, and pointer holds
  admission state: active or suspended by the cap
  runtime state: effect-specific mutable DSP state
```

Runtime state should be fixed-size except for histories allocated during
`reset`. Prefer enums or concrete structs over audio-thread dynamic dispatch.
Sharing functions and lookup tables is encouraged. Sharing mutable state
between slots is not.

Do not implement direct keys by writing or clearing the automatable trigger
parameter. Preserve independent source-held state and aggregate sources before
admission so releasing one source cannot release a slot still held by another.

### 6.2 Sample flow

The conceptual loop is:

```text
signal = host_input_sample

for slot in deterministic_slot_order:
    stage_input = signal

    if slot is configured as a buffer effect:
        record stage_input into that slot's prepared history

    if slot is an admitted continuous processor:
        signal = process_slot(slot, stage_input)

host_output_sample = signal
```

The implementation may process blocks or specialize paths, but its output and
state transitions must match this ordering. Recording must not overwrite a
sample that the same stage still needs to read; tests must cover ring wrap and
minimum lookback.

### 6.3 Why bypassed buffer stages matter

For a buffer effect to produce instant lookback on the first press, it must
already contain the signal that arrived at its own chain position. A downstream
buffer stage therefore records the result of upstream **active** stages, while
an upstream buffer stage records a less-processed signal. One shared input ring
cannot reproduce both histories.

If an inactive buffer stage does not record continuously, first-press lookback
must be defined as cold, dry-source-derived, or otherwise degraded. That can be
a valid product decision, but it is not exact serial lookback and must be made
visible at G1.

### 6.4 Active-cap state machine

The cap applies to admitted continuous processors, not physical key state and
not pitch actions. Keep at least these concepts separate:

- `held`: requested by keyboard, MIDI, automation, or pointer;
- `active`: admitted to the audible serial chain;
- `suspended`: still held but excluded by the cap.

The candidate policy to evaluate is:

1. Admit a newly held continuous slot if capacity exists.
2. If full, suspend the least-recently admitted active slot and admit the new
   one.
3. When capacity returns, restore still-held suspended slots by recency.
4. Regardless of admission recency, process the active subset in ascending
   slot order.

Do not conflate the admission order with the audio-chain order. Tests must
cover presses and releases in the same block, multi-source holds on one slot,
focus loss, MIDI on different channels, automation plus MIDI, and action slots
pressed while the chain is full.

## 7. Keyboard-first without losing compatibility

Keyboard-first is a product interaction and visual-hierarchy decision, not a
reason to remove protocols.

Retain three layers:

1. performance slots and their configs/state in the DSP/product model;
2. input adapters for pointer, automation, fixed MIDI notes, and optional
   direct computer keyboard;
3. a keyboard-shaped editor surface that makes computer keys legible.

The existing Slint backport maps baseview keyboard events to Slint using
logical key text, distinguishes repeated presses, and can return the event to
the parent host. It does not establish physical scan-code mapping. With
passthrough enabled, a direct-key handler and a DAW virtual MIDI keyboard may
both react to one computer-key event. The keyboard research node must therefore
test, not assume:

- logical character versus physical position across keyboard layouts;
- repeat suppression;
- release delivery and “all keys up” on focus loss/editor close;
- modifier and text-control behavior;
- host shortcut capture;
- double triggering through host virtual MIDI;
- CLAP, VST3, and standalone differences on macOS, Windows, and Linux.

Do not expand `vendor/truce-slint` with product behavior. A framework patch is
allowed only if the needed generic event capability is missing, is narrowly
scoped, has its own tests and `PATCH.md` update, and is justified in the ADR.

## 8. Target ownership boundaries

Preserve the repository's established boundaries:

| Concern | Owner |
| --- | --- |
| serial chain, admission resolver, per-slot state, buffer histories, DSP metadata | `dsp/src/lib.rs` (split internally only if the owner approves a focused DSP module refactor) |
| factory configurations and native kit codec | `kit/src/lib.rs` |
| host parameter schema, persisted editor metadata, atomics for held/active/suspended/visualization | `src/params.rs` |
| MIDI note input and per-channel held state | `src/midi.rs` |
| TRUCE process integration, pitch action events, prepared scratch | `src/lib.rs` |
| direct-key bindings, automation-safe editor callbacks | `src/editor.rs` |
| keyboard-shaped visual surface and chain status | `ui/main.slint` |
| generic Slint/baseview keyboard capability only, if proven necessary | `vendor/truce-slint` |

No host, UI, filesystem, or TRUCE types may enter `dsp` or `kit`.

## 9. JCode task DAG

Use deep mode because this is a risky, cross-cutting architecture and contract
change. Keep the initial live-worker budget modest (about 6–8); the bottleneck
is evidence and integration, not maximum agent count. JCode detects shifting
files but its task graph does not provide mutual exclusion. Never assign two
concurrent writers to the same file.

### Phase R — parallel, read-only exploration

Seed node `R` with kind `explore`; its owner must immediately call `expand_node`
for children R0–R6, which makes `R` composite at runtime. Deep mode inserts and
enforces the critique gate; `RC` is shown
for readability but must not be seeded as an ordinary node. After all children,
gate-created gap nodes, and the gate pass, the `R` owner synthesizes the G1 memo
as `R`'s typed completion artifact. `RS` is that owner synthesis step, not a
separate terminal-action kind.

| ID | Supported kind | Task | Depends on | Required artifact |
| --- | --- | --- | --- | --- |
| R | `explore` | own Phase R decomposition and synthesize the G1 decision memo | — | recommendations, evidence, unresolved decisions, proposed ADR outline |
| R0 | `explore` | preflight: repository status, instructions, authoritative docs, relevant ADRs | — | clean/dirty state, constraints, exact current contracts |
| R1 | `explore` | map current DSP selection, effect state, processing order, transition semantics, and regression tests | R0 | file/line evidence and behavioral model |
| R2 | `explore` | model exact serial buffer semantics and candidate history strategies | R0 | correctness analysis, memory formulas, degradation modes |
| R3 | `explore` | establish CPU/memory benchmark plan and current baseline at 44.1/48/96/192 kHz | R0 | reproducible commands and measurements |
| R4 | `explore` | test TRUCE/Slint direct keyboard feasibility and host/passthrough risks | R0 | platform/format findings; minimal throwaway test if needed |
| R5 | `explore` | enumerate parameter, MIDI, kit, state, visualization, fixture, UI, and documentation blast radius | R0 | contract-change matrix |
| R6 | `explore` | translate the musical intent into testable interaction semantics; identify ambiguities | R0 | state tables and owner questions |
| RC | automatic critique gate | adversarially audit R1–R6 and inject gap nodes for unsupported or missed material | R1–R6 | enforced gate artifact covering its complete scope |
| RS | composite-owner synthesis | produce `R`'s G1 decision-memo completion artifact | RC and all gap nodes | `R` completion artifact |

### Gate G1 — owner decision, no production code before it

G1 resolves every item in section 4. The coordinator should present decisions
as one coherent product bundle, because slot count, cap, buffer policy, and
keyboard behavior affect one another.

G1 is a human continuation boundary, not an executable worker node or a DAG
dependency. Seed and run only the Phase R composite initially. Present the
decision memo only after `R` has synthesized its artifact and the deep plan's
automatic root gate has passed. Once the owner supplies every G1 decision, keep
the same coordinator/session and re-seed Phase D with `task_graph` into the
existing plan. Re-seeding widens and reopens the automatic root gate. Do not use
`expand_node` as a general append operation and do not create a replacement
swarm.

### Phase D — accepted architecture and contracts

| ID | Kind | Task | Depends on | Required artifact |
| --- | --- | --- | --- | --- |
| D1 | implement | draft a new ADR for serial performance slots, state ownership, cap/admission policy, buffer history policy, keyboard policy, and schema decision; do not land it separately | recorded owner decisions | reviewed ADR draft |
| D2 | explore | specify the exact authoritative contract/architecture/roadmap edits that must accompany implementation; do not edit those current-state documents yet | D1 | documentation change plan consistent with ADR |
| D3 | verify | audit D1 and the D2 change plan against the recorded owner decisions and repository boundaries | D1–D2 | pass/fail; fix nodes for mismatches |

The reviewed ADR draft records the accepted decision before code agents invent APIs. The
authoritative current-state documentation specified by D2 is applied by IC only
after I4–I6 are integrated. IC lands the ADR, implementing code and tests, and
authoritative documentation together. None may land separately in a state that
describes behavior the repository does not yet implement.

### Phase I — implementation

Serialize I1–I3 because they own the same DSP core.

| ID | Kind | Task | Depends on | Exclusive write scope | Done contract |
| --- | --- | --- | --- | --- | --- |
| I1 | implement | introduce per-slot runtime state and the held/active/suspended admission resolver; keep pitch actions edge-triggered | D3 | `dsp/src/lib.rs` | transition and chain-order unit tests pass; process path remains bounded |
| I2 | implement | convert Gate, bands, and LoFi to independent serial stage processing, including same-type stacking | I1 | `dsp/src/lib.rs` | one-stage equivalence where intended plus order/non-commutativity/duplicate-type tests |
| I3 | implement | implement the accepted per-slot buffer-history strategy and serial Repeat/Reverse/Tape Stop stages | I2 | `dsp/src/lib.rs` | first-press, ring-wrap, upstream/downstream capture, same-type, release/retrigger, and memory tests pass |
| I4 | implement | adapt wrapper state translation, pitch events, MIDI aggregation, atomics, and parameter schema as required | I3 | `src/lib.rs`, `src/midi.rs`, `src/params.rs` | wrapper/MIDI/parameter tests pass; no process allocation |
| I5 | implement | update kit codec/factory kits and create a new fixture version for intentional sound changes | I3 | `kit/src/lib.rs`, `contract/` | codec rejects wrong layouts; fixtures are versioned, never silently rewritten |
| I6 | implement | add accepted direct-key input behavior, keyboard-shaped UI, active/suspended feedback, and multi-buffer visualization selection | I4 | `src/editor.rs`, `ui/main.slint`; vendor only under section 7 rules | UI tests/screenshots and focus/release behavior pass |
| IC | implement | integrate I4–I6 after the DSP API commit, apply the D2 authoritative documentation plan, resolve cross-layer inconsistencies, and commit ADR/code/tests/docs together | I4–I6 and D2 | integration commit, changed-contract summary |

I4 and I5 can run in parallel after I3. I6 depends on the wrapper/atomic API in
I4. If slot count remains 16, preserve current parameter IDs unless G1/ADR says
otherwise. If the count changes, calculate the new schema (`1 + 9N`), address
all `u16` assumptions, and follow the ADR's explicit no-migration or authorized
migration decision.

### Phase V — verification and correction

| ID | Kind | Task | Depends on | Required checks |
| --- | --- | --- | --- | --- |
| V1 | verify | focused semantic tests | IC | serial order, duplicate types, local wet/dry, cap overflow/restoration, actions at capacity, multisource holds, kit reset, state restore |
| V2 | verify | realtime and resource verification | IC | `rt-paranoid`; benchmark CPU and memory across sample rates, block sizes, slot counts, active counts, and buffer-heavy kits |
| V3 | verify | full repository automated acceptance | IC | formatting, Clippy, workspace tests, corpus integrity, CLAP/VST3 build |
| V4 | verify | editor visual acceptance | IC | every committed screenshot size/state plus new keyboard/active/suspended/multi-buffer states |
| V5 | verify | real-host acceptance | V3–V4 | relevant REAPER checklist for CLAP/VST3: focus, virtual MIDI, direct keys, automation, state, tempo/buffer-size changes, lifecycle |
| V6 | verify | adversarial final contract and architecture audit | V1–V5 | no undocumented behavior, no legacy dependency, no shared mutable stage state, no hidden degradation |

After every V node and any spawned fix/reverify node passes, the coordinator
writes the final delivery report outside the task graph's terminal-action kinds:
shipped behavior, commits, commands/results, benchmark table, manual checks,
known limitations, and `what_i_did_not_check`.

A failing verify node creates a focused fix node and re-runs the failed verifier.
No verifier may downgrade a failure to prose.

## 10. Required test properties

At minimum, add tests proving:

1. Two held slots of the same exact effect type both alter the signal.
2. Swapping configured slot numbers changes output for non-commutative stages,
   while swapping press order does not change chain order.
3. Every stage mixes against its own input.
4. A downstream buffer captures upstream processed audio; an upstream buffer
   does not capture downstream processing.
5. Two serial buffer stages do not read the same shared mutable history.
6. Held, active, and suspended states transition according to the accepted cap
   policy, including restoration.
7. Pitch actions fire once per aggregate released-to-held transition and do
   not consume the cap.
8. Multiple MIDI channels and simultaneous MIDI/automation/pointer/keyboard
   sources do not release one another prematurely.
9. Key repeat does not retrigger unless G1 explicitly chooses retriggering.
10. Focus loss, editor close, plugin deactivation, kit changes, and host state
    restoration cannot leave stuck direct-key holds.
11. `process` allocates nothing under worst-case overlap.
12. Output stays finite across all effects, maximum feedback/drive-like
    settings introduced later, and pathological input.
13. Reset memory is deterministic and rejected or degraded explicitly if an
    accepted resource ceiling cannot be met.

Intentional serial sound changes require a new fixture version. Keep the frozen
existing corpus and its checksums intact as evidence of the previous canonical
sound.

## 11. Required commands before delivery

```sh
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
cargo test --locked --workspace
cargo test --locked --workspace --features rt-paranoid
(cd contract && shasum -a 256 -c SHA256SUMS)
cargo truce build --clap --vst3
```

Run the screenshot commands and relevant DAW checklist from `docs/TESTING.md`.
Benchmark with at least five like-for-like runs per case and report medians plus
range; do not present one run as a capacity conclusion.

## 12. Copy/paste coordinator prompt

Start JCode in the repository, enable swarm if it is disabled, and give the
root session this prompt:

```text
Use JCode swarm-deep and its DAG-first workflow for this risky cross-cutting
change. Read AGENTS.md and docs/SERIAL_KEYBOARD_SWARM_BRIEF.md completely, then
read every repository source named there. Treat the brief as the controlling
proposal and the current CONTRACTS/ADRs as current truth.

Seed Phase R as node R with kind explore, then immediately call expand_node for
the parallel read-only R0-R6 children so R becomes composite at runtime. Rely on
deep mode's automatic critique gate. Have the Phase R owner synthesize the G1
decision memo as its typed completion artifact. Do not edit
production code before G1. Do not invent unresolved choices: slot count,
keyboard mapping/policy, active cap and overflow, buffer-history policy,
effect-type-change behavior, visualization selection, and runtime-state
lifecycle require owner approval.

After the Phase R artifact is synthesized and the automatic root gate passes,
present the G1 memo. After I provide the G1 decisions, re-seed Phase D into this
existing plan with `task_graph`, which reopens the root gate, then enact Phases
D, I, and V. Serialize agents
that touch dsp/src/lib.rs and never allow concurrent writers to one file. Keep
MIDI, automation, and pointer compatibility; keep DSP framework-neutral f64;
obey all real-time constraints; do not use archived C++/WRAC code or add legacy
migration without the new ADR. A failing critique/verify creates gap/fix nodes
and re-runs the gate. Finish only with all required checks passing and a report
containing commits, validation, benchmark evidence, manual checks, limitations,
and what was not checked. Treat G1 as a human continuation boundary, not a node.
Do not use `expand_node` as a general append operation. Keep direct-key held state
independent from automatable trigger state so one input source cannot release
another. Draft the ADR before implementation, but land ADR/code/tests/current-
state documentation together in IC.
```

The prompt intentionally pauses at G1. After the decision memo, append the
owner's answers to section 4 or send them to the same coordinator, then tell it
to continue the existing task graph rather than starting over.

## 13. Research sources

Primary sources consulted for the harness and character reference:

- [JCode repository and swarm overview](https://github.com/1jehuang/jcode)
- [JCode swarm architecture](https://github.com/1jehuang/jcode/blob/master/docs/SWARM_ARCHITECTURE.md)
- [JCode DAG-first swarm task graph](https://github.com/1jehuang/jcode/blob/master/docs/SWARM_TASK_GRAPH.md)
- [JCode user documentation](https://jcode.sh/docs)
- [Sherman official downloads and FilterBank 2 manual](https://sherman.be/index.php/support/downloads)
- [Sherman FilterBank 2 official manual page](https://www.sherman.be/index.php/support/downloads/item/sherman-filterbank-v2-manual-english)
- [Sherman official FilterBank 2 addendum](https://www.sherman.be/index.php/support/downloads/item/fb2-manual-addendum)

Repository claims in this brief come from the canonical local source and
documentation named in sections 5 and 8, not from archived implementations.
