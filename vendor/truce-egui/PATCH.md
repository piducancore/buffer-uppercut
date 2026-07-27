# Local `truce-egui` backport

This directory contains the published `truce-egui` 6.3.0 crate from
crates.io with one narrowly scoped host-integration fix.

The upstream event handler reports every keyboard event as captured, even
when egui has no focused text widget. A parented plugin window therefore
swallows REAPER's computer-keyboard events before the virtual MIDI keyboard
can translate them into notes. This backport returns `EventStatus::Ignored`
when `egui_wants_keyboard_input()` is false, while preserving capture for
text editing.

Remove the `[patch.crates-io]` override and this directory after the behavior
ships in the upstream TRUCE release used by this project.
