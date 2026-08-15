# Local `truce-slint` backport

This directory contains the published `truce-slint` 6.3.0 crate from
crates.io with narrowly scoped editor-input additions.

## Host keyboard passthrough

`SlintEditor::keyboard_passthrough(bool)` opts an editor into returning
mapped keyboard events to its parent host after dispatching them to Slint.
The default remains `false`, preserving upstream capture behavior.

Buffer Uppercut enables passthrough so a parented plug-in window does not
prevent REAPER's computer keyboard and virtual MIDI keyboard from seeing
key presses. Pointer events and unmapped keyboard events are unchanged.
The iOS builder exposes the same method as a no-op because that backend
currently receives touch input only.

## Physical keyboard callback

`SlintEditor::physical_keyboard_input(...)` exposes native physical key codes,
repeat state, focus changes, and editor closure to a generic callback. Returning
`true` consumes that key event even when keyboard passthrough is enabled. The
callback does not implement product mappings, and logical events still reach
Slint before an unconsumed event is returned to the parent host. The iOS builder
accepts the callback as a no-op for API parity.

Remove the `[patch.crates-io]` override and this directory after equivalent
passthrough and physical-key callback behavior ships in the upstream TRUCE
release used by this project.
