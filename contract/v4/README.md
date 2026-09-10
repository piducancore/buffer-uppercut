# Buffer Uppercut DSP regression corpus v4: held grain Pitch

`dsp-contract-v4-grain-pitch` is the current audio corpus for the held grain
Pitch workflow approved in
[`docs/adr/0009-held-grain-pitch.md`](../../docs/adr/0009-held-grain-pitch.md).

The v3 processing scenarios were migrated to the consolidated Pitch, Filter,
LoFi, and Vinyl numbering. Former Pitch Down and Pitch Up configurations became
action roles. Former Pitch Reset configurations became Trigger roles. Expected
outputs were captured from the current Rust engine, including the separation of
Beat Repeat's Slice Pitch from the active Pitch shift.

Processing files use `BUDSP_CONTRACT\t4`. Pitch actions use
`BUDSP_PITCH_ACTIONS\t4`. Outputs must be finite and satisfy:

```text
abs(actual - expected) <= 1e-7 + 1e-7 * abs(expected)
```

Verify from the repository root:

```sh
(cd contract/v4 && shasum -a 256 -c SHA256SUMS)
cargo test -p buffer-uppercut-dsp --test contract
```

Regenerate only for this approved change while v4 remains current:

```sh
cargo run -p buffer-uppercut-dsp --example regenerate_contract_v4
```
