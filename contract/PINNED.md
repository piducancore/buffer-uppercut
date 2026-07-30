# Seed corpus provenance

- Contract: `dsp-contract-v1`
- Reference C++ commit: `bc17659aa517b9910761c1861cadd873402b75de`
- `manifest.json` SHA-256:
  `0a126f1ce3159cf2753c2b6dae08f2a99396d5f9f066eda61409bc74b59e0c54`

Verify the vendored snapshot from the repository root:

```sh
(cd contract && shasum -a 256 -c SHA256SUMS)
cargo test -p buffer-uppercut-dsp --test contract
```

The manifest contains the SHA-256 of each fixture. These files are now owned as
regression evidence by the canonical Rust repository. Do not resync them from
an archived implementation. Intentional sound changes create a new versioned
corpus; never update individual expected samples silently.
