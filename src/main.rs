//! Entry point for the development-only standalone host - run the plugin as a
//! regular desktop app via `cargo truce run`, no DAW needed. It is not a
//! supported or distributed product format. Only compiled
//! when the `standalone` feature is enabled (see `[[bin]]` in
//! Cargo.toml).
//!
//! Keep this host isolated from release packaging until its audio-input and
//! monitoring experience is deliberately specified as product behavior.

use buffer_uppercut::Plugin;

fn main() {
    // `run::<Plugin>()` parses argv + `TRUCE_STANDALONE_*` env vars
    // and dispatches. To pin launch defaults (e.g. mic on at start
    // for an effect demo) call `run_with::<Plugin>(Defaults { … })`
    // - argv / env still take precedence over the values you pass.
    truce_standalone::run::<Plugin>();
}
