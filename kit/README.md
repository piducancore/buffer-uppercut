# Buffer Uppercut kits

This crate owns product factory kits and the native `.bupreset` version 1
codec. It is framework-neutral and performs no file I/O.

The editor is responsible for selecting files and applying decoded values
through host automation. The audio wrapper is responsible for clearing
transient DSP state after a kit change.

See [`../docs/CONTRACTS.md`](../docs/CONTRACTS.md) for the wire layout and
[`../docs/ARCHITECTURE.md`](../docs/ARCHITECTURE.md) for the application flow.
