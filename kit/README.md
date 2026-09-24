# Buffer Uppercut kits

This crate owns product factory kits, validated physical-key maps, and the native
`.bupreset` version 3 codec. It is framework-neutral and performs no file I/O.

The editor applies factory configurations through host automation. Host state
and native kits carry the same durable sound values, name, and key map; transient
holds and Active Shift are excluded from native kits and reset on host recall.
The audio wrapper clears transient DSP state after a kit change. There is no
embedded native-file import/export UI; codec support is not a preset browser.

Version 3 appends sixteen stable key identifiers to the sound payload and
rejects versions 1 and 2. `keys.rs` owns identifier validation, labels, unique
assignments, and explicit swaps; platform key conversion belongs to the wrapper.

See [`../docs/CONTRACTS.md`](../docs/CONTRACTS.md) for the wire layout and
[`../docs/ARCHITECTURE.md`](../docs/ARCHITECTURE.md) for the application flow.
