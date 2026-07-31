# ADR 0002: Slint is the production editor

- Status: accepted
- Date: 2026-07-30

## Context

The project evaluated WebView/React and egui interfaces. Web UI offered
excellent ecosystem familiarity but required a serialized command/event
boundary and a development server workflow. Egui was concise for tools but did
not naturally express the desired polished, responsive product layout.

## Decision

Use Slint as the only editor implementation. Keep declarative visual structure
in `ui/main.slint` and host/automation/file bindings in `src/editor.rs`.

Continuous controls emit explicit begin/set/end gestures. Unused keyboard
events pass through to the host. The editor remains resizable with deterministic
size baselines. Complete plugin presets use the host-native preset UI provided
through the TRUCE wrappers. The embedded editor does not launch duplicate
platform preset dialogs.

## Consequences

- UI bindings remain strongly typed and native.
- There is no WebView runtime or frontend build server.
- Slint-specific host integration must be tested for focus, DPI, resizing, and
  lifecycle behavior.
- Custom editor changes require rebuilding and reopening the editor.

## Rejected alternatives

- React/WebView: larger bridge and runtime surface for this product.
- Egui: appropriate for tooling, less suitable for the selected visual system.
- Multiple feature-gated editors: doubles integration and acceptance work.
