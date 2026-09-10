# Regression corpus provenance

## V1 seed evidence

- Contract: `dsp-contract-v1`
- Location: root `contract/*.budsp`
- Reference C++ commit: `bc17659aa517b9910761c1861cadd873402b75de`
- `manifest.json` SHA-256:
  `0a126f1ce3159cf2753c2b6dae08f2a99396d5f9f066eda61409bc74b59e0c54`

V1 remains byte-for-byte frozen. It records the experimental reference sound but
is not the current serial engine oracle.

## V2 frozen serial corpus

- Contract: `dsp-contract-v2-serial`
- Location: `contract/v2/`
- Reference engine: canonical Rust `dsp/src/lib.rs`
- Decision: `docs/adr/0004-serial-performance-slots.md`
- Expected-output storage boundary: `f64` processing with configured stereo
  histories stored as `f32`

V2 is pinned by its own `manifest.json` and `SHA256SUMS`. It retains the broad v1
input scenarios with serial outputs and adds focused serial/cap/history cases.

## V3 frozen unified Filter corpus

- Contract: `dsp-contract-v3-unified-filter`
- Location: `contract/v3/`
- Reference engine: canonical Rust `dsp/src/lib.rs`
- Decision: `docs/adr/0008-unified-filter-effect.md`

V3 is pinned by its own manifest and checksum list.

## V4 current held grain Pitch corpus

- Contract: `dsp-contract-v4-grain-pitch`
- Location: `contract/v4/`
- Reference engine: canonical Rust `dsp/src/lib.rs`
- Decision: `docs/adr/0009-held-grain-pitch.md`

Verify all snapshots from the repository root:

```sh
(cd contract && shasum -a 256 -c SHA256SUMS)
(cd contract/v2 && shasum -a 256 -c SHA256SUMS)
(cd contract/v3 && shasum -a 256 -c SHA256SUMS)
(cd contract/v4 && shasum -a 256 -c SHA256SUMS)
cargo test -p buffer-uppercut-dsp --test contract
```

The canonical contract test executes v4. V1–V3 remain separately verifiable
historical evidence. Do not resync a frozen corpus or update individual expected
samples silently. A later deliberate sound change requires a reviewed new
contract version once the affected corpus is frozen.
