# DSP Contract Deviations

There are no approved deviations from the frozen expected output in
`dsp-contract-v1`.

Any deliberate Rust behavior change must be reviewed before implementation and
recorded here with:

- the affected fixture and behavior;
- the motivation and audible consequence;
- the independently established expected result;
- the contract version that adopts the change; and
- confirmation that the canonical DSP, wrapper, and documentation agree.

Never rewrite a versioned fixture silently. A behavior change creates a new
contract version.
