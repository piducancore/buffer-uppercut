# AGENTS.md

## Product direction

This repository is the sole canonical implementation of **Buffer Uppercut**.
It uses TRUCE 6.3, a native Slint editor, and a framework-neutral Rust `f64`
DSP core. Earlier C++ and WRAC repositories are archived experiments, not
compatibility targets or implementation dependencies.

Do not add legacy state migration, old product identities, cross-port preset
compatibility, or code copied from an experimental port unless a new decision
record explicitly authorizes it.

## Start here

Before changing code:

1. Read `README.md`.
2. Read `docs/ARCHITECTURE.md` and `docs/CONTRACTS.md`.
3. Read `docs/ROADMAP.md` for current status and open acceptance work.
4. Read the relevant file in `docs/adr/` when changing architecture.
5. Run `git status --short`; preserve all existing user changes.

## Ownership boundaries

- `dsp/src/lib.rs`: framework-neutral DSP, performance state, effect behavior,
  semantic control metadata, and visualization extraction.
- `kit/src/lib.rs`: factory kits and native `.bupreset` serialization.
- `kit/src/keys.rs`: framework-neutral stable key identifiers and validated maps.
- `src/params.rs`: the 144 host parameters at stable IDs `1..144`, persisted editor metadata, and
  atomic UI snapshots.
- `src/midi.rs`: MIDI note mapping and per-channel held-state updates.
- `src/keyboard.rs`: physical-key conversion.
- `src/input.rs`: local key/pointer ownership and host performance gestures.
- `src/lib.rs`: TRUCE `PluginLogic64`, host events, transport, preallocated
  wrapper buffers, and DSP integration.
- `src/editor.rs`: automation-safe Slint bindings and UI-thread file access.
- `ui/main.slint`: declarative visual components and responsive layout.
- `truce.toml`: final plugin identity and format metadata.
- `vendor/`: narrow version-locked framework patches. Read each `PATCH.md`
  before editing and remove a patch when upstream contains equivalent behavior.

Do not move host, UI, filesystem, or TRUCE types into `dsp` or `kit`.

## Real-time rules

Inside `PluginLogic::process` and anything it calls:

- no allocation or resizing;
- no locks;
- no logging;
- no filesystem access;
- no blocking or unbounded work.

Allocate in `reset`. Communicate with the UI through atomics or host events.
Any process-thread allocation reported by `rt-paranoid` blocks acceptance.

## Contract changes

The current parameter schema, MIDI map, kit format, state semantics, and
block-boundary event timing are documented in `docs/CONTRACTS.md`. They may
change while the product is unreleased, but only deliberately: update code,
tests, documentation, and an ADR together. There is no obligation to preserve
state or identity from the archived experiments.

## Required verification

Before handing off an implementation:

```sh
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
cargo test --locked --workspace
cargo test --locked --workspace --features rt-paranoid
cargo truce build --clap --vst3
```

For UI changes, check all three committed screenshots. For wrapper, MIDI,
state, kit, or editor changes, complete the relevant DAW checklist in
`docs/TESTING.md`.

## Documentation rule

Documentation describes the current repository, not conversation history.
When behavior, ownership, commands, product identity, or milestone status
changes, update the authoritative document in the same change. Avoid repeating
the same rule in several files; link to its source of truth instead.
