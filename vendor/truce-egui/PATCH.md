# Local `truce-egui` backport

This directory contains the published `truce-egui` 6.3.0 crate from
crates.io with two narrowly scoped additions.

## REAPER keyboard passthrough

The upstream event handler reports every keyboard event as captured, even
when egui has no focused text widget. A parented plugin window therefore
swallows REAPER's computer-keyboard events before the virtual MIDI keyboard
can translate them into notes. This backport returns `EventStatus::Ignored`
when `egui_wants_keyboard_input()` is false, while preserving capture for
text editing.

## Editor-only knob value text

`param_knob_with_value_text` preserves the stock knob's parameter reads and
host automation gesture protocol while allowing a product editor to override
only the value string painted below the knob. Buffer Uppercut uses this for
effect-contextual values such as grid divisions, semitones, milliseconds, and
bits without changing the host-facing parameter formatter or parser.

Remove the `[patch.crates-io]` override and this directory after the behavior
ships in the upstream TRUCE release used by this project, or move the
editor-only knob wrapper into the product crate if upstream chooses not to
expose an equivalent API.
