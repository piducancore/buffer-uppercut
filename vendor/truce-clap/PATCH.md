# Local `truce-clap` backport

This directory contains the published `truce-clap` 6.3.0 crate from
crates.io, with one narrowly scoped CLAP compliance backport.

Buffer Uppercut calls `clap_host_params.rescan(CLAP_PARAM_RESCAN_VALUES)`
after the wrapper synchronously restores parameter values during state and
preset loads. This keeps the host's parameter cache coherent and satisfies
the CLAP state-reproducibility contract.

Remove the `[patch.crates-io]` override and this directory after the behavior
ships in the upstream TRUCE release used by this project.
