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
  governs Low Band, Mid Band, and High Band topology, modulation, nonlinear
  safety, macro semantics, and per-slot state.
