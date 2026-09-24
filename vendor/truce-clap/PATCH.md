# Local `truce-clap` backport

This directory contains the published `truce-clap` 6.3.0 crate from
crates.io, with narrowly scoped CLAP compliance backports.

Buffer Uppercut calls `clap_host_params.rescan(CLAP_PARAM_RESCAN_VALUES)`
after the wrapper synchronously restores parameter values during state and
preset loads. This keeps the host's parameter cache coherent and satisfies
the CLAP state-reproducibility contract.

Persistent non-parameter editor edits can additionally call the core
`mark_state_dirty` API. An atomic pending flag and `request_callback` marshal
the optional `clap_host_state.mark_dirty` call onto the host main thread.
This does not load state or manufacture parameter automation.

Remove the `[patch.crates-io]` override and this directory after the behavior
ships in the upstream TRUCE release used by this project.
