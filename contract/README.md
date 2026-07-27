# Buffer Uppercut DSP Contract v1

This directory is the frozen, framework-neutral behavior contract for the C++
engine at commit `bc17659aa517b9910761c1861cadd873402b75de`. WRAC and TRUCE
may consume these files, but neither Rust implementation is an oracle for the
other.

Each `.budsp` file is UTF-8, tab-separated text. A processing fixture contains:

- `sample_rate` and `channels` metadata;
- all 16 pad effect numbers and seven normalized macros;
- ordered blocks with tempo, a 16-bit held-pad mask, performance pitch, and
  frame count; and
- planar input and expected output samples as round-trippable decimal `f64`.

State changes occur at block boundaries. Mono fixtures omit right-channel
values and pass a null right input/output to the C++ engine. Implementations
must produce finite samples within:

```text
abs(actual - expected) <= 1e-7 + 1e-7 * abs(expected)
```

`pitch-actions.budsp` separately contracts the wrapper-triggered pitch-down,
pitch-reset, and pitch-up operations.

Fixture checksums live in `manifest.json`. See
[`../../DEVELOPMENT.md`](../../DEVELOPMENT.md) for regeneration and verification
commands. Changes to expected behavior require a new contract version and an
entry in `DEVIATIONS.md`.
