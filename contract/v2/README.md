# Buffer Uppercut DSP regression corpus v2: serial

`dsp-contract-v2-serial` is the canonical audio corpus for the serial
performance-slot architecture approved in
[`docs/adr/0004-serial-performance-slots.md`](../../docs/adr/0004-serial-performance-slots.md).

## Provenance

The 19 numbered scenarios inherited from v1 retain the same effect
configurations, block timing, held masks, tempo, pitch, and input samples. Their
expected outputs were captured from the canonical Rust serial engine after the
approved topology and lifecycle change. V1 remains frozen in the parent
directory.

Four serial-specific scenarios supplement that broad input set:

- `19-serial-duplicate-gates.budsp`: repeated exact effect types are independent
  serial stages;
- `20-six-stage-cap-suspend-restore.budsp`: new requests suspend the oldest
  admitted active processor at the cap and restore still-held requests in the
  approved order;
- `21-buffer-history-suspend-release.budsp`: suspension freezes processor state,
  release resets it, and configured buffer history continues through both; and
- `22-buffer-history-chain-position.budsp`: buffer lookback is captured from the
  signal entering that slot after lower-numbered active stages.

The v2 samples complement focused DSP unit tests that inspect admission masks,
processor state, and history positions directly. The corpus is pinned by
`manifest.json` and `SHA256SUMS`; it is not regenerated during tests.

## Format and tolerance

Processing files use `BUDSP_CONTRACT\t2`. Pitch actions use
`BUDSP_PITCH_ACTIONS\t2`. State changes occur at block boundaries. Expected
outputs are round-trippable decimal `f64` values produced by `f64` processing;
configured histories use the architecture's deliberate `f32` storage boundary.

Outputs must be finite and satisfy:

```text
abs(actual - expected) <= 1e-7 + 1e-7 * abs(expected)
```

Verify from the repository root:

```sh
(cd contract/v2 && shasum -a 256 -c SHA256SUMS)
cargo test -p buffer-uppercut-dsp --test contract
```

Any later intentional sound change must be reviewed with code, focused tests,
authoritative documentation, and an ADR decision. Create a new corpus version
rather than silently replacing these expected outputs once v2 is frozen for
release.
