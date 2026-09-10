# Buffer Uppercut DSP regression corpora

This directory contains four deliberately separated contract versions.

## V1: frozen seed evidence

The root `.budsp` files, `manifest.json`, and `SHA256SUMS` are the frozen
`dsp-contract-v1` corpus seeded from the experimental C++ engine at commit
`bc17659aa517b9910761c1861cadd873402b75de`. Its provenance establishes how
those expected samples were produced. It does not make the archived
implementation a compatibility target or perpetual oracle.

V1 is retained unchanged as historical evidence. The approved serial topology
intentionally changes some outputs, so the canonical Rust engine is not required
to match v1 audio. Never rewrite root v1 fixtures or their checksums to make a
new architecture pass.

## V2: frozen serial expectations

[`v2/`](v2/) contains `dsp-contract-v2-serial`, the canonical regression corpus
for ADR 0004. It keeps the broad v1 input scenarios with serial-engine outputs
and adds focused fixtures for duplicate same-type stages, the six-stage cap and
restore policy, buffer history across suspension/release, and exact
chain-position lookback.

Each corpus owns its own manifest and checksum list:

```sh
(cd contract && shasum -a 256 -c SHA256SUMS)
(cd contract/v2 && shasum -a 256 -c SHA256SUMS)
(cd contract/v3 && shasum -a 256 -c SHA256SUMS)
(cd contract/v4 && shasum -a 256 -c SHA256SUMS)
cargo test -p buffer-uppercut-dsp --test contract
```

## V3: frozen unified Filter expectations

[`v3/`](v3/) contains `dsp-contract-v3-unified-filter`. It migrates the v2
inputs to one Filter effect with Low-pass, Band-pass, and High-pass modes and
captures the Rust engine output for that milestone.

## V4: current held grain Pitch expectations

[`v4/`](v4/) contains `dsp-contract-v4-grain-pitch`. It consolidates Pitch into
Down, Trigger, and Up roles, includes the tempo-preserving grain processor, and
renumbers Filter, LoFi, and Vinyl. The DSP contract harness executes v4; V1–V3
remain checksum-only evidence.

## Fixture format

Each processing fixture is UTF-8, tab-separated text containing:

- a versioned `BUDSP_CONTRACT` header;
- `sample_rate` and `channels` metadata;
- all 16 slot effect numbers and seven normalized macros;
- ordered blocks with tempo, a 16-bit held-slot mask, performance pitch, and
  frame count; and
- planar input and expected output samples as round-trippable decimal `f64`.

State changes occur at block boundaries. Mono fixtures omit right-channel
values. Implementations must produce finite samples within:

```text
abs(actual - expected) <= 1e-7 + 1e-7 * abs(expected)
```

`pitch-actions.budsp` separately contracts role-driven Down and Up step
operations, including Trigger's no-op action behavior.

See [`DEVIATIONS.md`](DEVIATIONS.md) for the approved v1-to-v2 change and
[`../docs/TESTING.md`](../docs/TESTING.md) for acceptance procedures.
