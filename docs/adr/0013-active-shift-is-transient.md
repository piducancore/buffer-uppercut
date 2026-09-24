# ADR 0013: Active Shift is transient performance state

- Status: accepted
- Date: 2026-09-23
- Implementation: current canonical architecture

## Context

ADR 0009 retained the existing Performance Pitch host parameter as the visible
and automatable carrier for the held grain Pitch gesture. The resulting value is
not durable configuration: Down and Up taps change it only while a Trigger is
held, and releasing the final Trigger returns it to zero. Nevertheless, hosts
could automate and persist the parameter, and native `.bupreset` version 1
stored it alongside durable pad configuration.

This made a transient result look like a preset choice and allowed host
automation or state recall to inject a shift independently of the hold-and-tap
gesture.

## Decision

Remove the Performance Pitch host parameter at ID `0` without renumbering the
sixteen slot groups at IDs `1..144`. Buffer Uppercut therefore exposes 144
automatable parameters and intentionally leaves ID `0` unused.

Keep the Down, Trigger, and Up interaction unchanged. The wrapper owns the
accumulated Active Shift as bounded transient state and publishes its current
integer value atomically to the editor's read-only indicator. It resets to zero
when the final Trigger releases and whenever activation, kit application, or
host-state restoration clears transient state. It is never automated or
persisted.

Remove the former Performance Pitch field from the native kit codec and advance
`.bupreset` to version 2. Version 1 files are intentionally rejected rather than
migrated. The format remains a strict encoding of durable kit configuration.

The framework-neutral DSP continues accepting Active Shift in
`PerformanceState`. This is block input from the wrapper and from the DSP
contract harness, not saved product configuration. The audio algorithm and v4
expected samples are unchanged.

This decision supersedes only the Performance Pitch parameter and kit-layout
parts of ADRs 0004 and 0009. Their slot IDs, Pitch roles, gesture, DSP, and
serial-chain decisions remain in force.

## Consequences

- Host automation lists no longer expose a control that the editor presents as
  read-only and the performance gesture immediately resets.
- Host state and native kits no longer recall a nonzero Active Shift.
- Existing slot automation IDs remain stable even though the parameter list has
  a deliberate gap at ID `0`.
- Native kit version 2 is eight bytes smaller than version 1 for the same name
  and slot configuration.
- UI publication uses only bounded atomic operations and does not add audio-thread
  allocation, locking, host events, or filesystem access.

## Rejected alternatives

- Keep parameter ID `0` but force it to zero during save: the host parameter
  would still advertise meaningless automation and presets could disagree with
  the editor's gesture semantics.
- Renumber slot parameters to `0..143`: this creates avoidable automation churn
  without changing product behavior.
- Remove the shared Active Shift gesture: the hold-Trigger and tap-Down/Up
  interaction remains intentional and useful.
- Keep a reserved pitch value in `.bupreset` version 1: a field with no product
  meaning would preserve the exact ambiguity this decision removes.
