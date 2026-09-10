# Architecture decision records

ADRs preserve decisions whose rationale future agents cannot reliably infer
from code.

Use the next sequential number and include:

- context;
- decision;
- consequences;
- rejected alternatives;
- status.

Do not use ADRs as task trackers or changelogs. Current work belongs in
`../ROADMAP.md`.

Current records:

- [0001 — TRUCE is the canonical implementation](0001-truce-canonical.md)
- [0002 — Slint is the production editor](0002-slint-editor.md)
- [0003 — Audio-to-UI data uses bounded atomic snapshots](0003-atomic-visualization.md)
- [0004 — Performance slots form a capped serial keyboard chain](0004-serial-performance-slots.md)
  governs the current serial DSP, admission, buffer-history, lifecycle, and
  direct-key architecture.
- [0005 — Performance bands use a bounded nonlinear resonant filter](0005-nonlinear-performance-filter.md)
  records the nonlinear topology that the unified Filter retains.
- [0006 — Vinyl is an independent record-wear effect](0006-vinyl-simulation.md)
  governs its macro semantics, short delay, and kit extension.
- [0007 — Editor type changes load effect defaults](0007-editor-effect-defaults.md)
  governs manual type selection, host defaults, and parameter-driven knob values.
- [0008 — Unify the performance filters behind a Mode control](0008-unified-filter-effect.md)
  governs Filter macros and modes, factory configurations, and the v3 DSP
  milestone; ADR 0009 supersedes its effect numbering.
- [0009 — Pitch uses held grain processing with action roles](0009-held-grain-pitch.md)
  governs Pitch roles and controls, active-shift gestures, current effect
  numbering, and the v4 DSP corpus.
