# 0012 — MIT original code and unsigned preview distribution

## Status

Accepted.

## Decision

License original Buffer Uppercut code under MIT, copyright 2026 piducancore.
Preserve upstream licenses for vendored and transitive dependencies. Use Slint's
royalty-free desktop option with its required public download-page attribution.

Allow deliberately reviewed unsigned CLAP/VST3 previews before trusted signing
is available. Use `-preview.N` versions and GitHub prerelease status; do not imply
stable acceptance, macOS notarization or Windows Authenticode trust. Standalone
remains excluded under ADR 0010.

The release workflow prepares drafts only, checks exact bundles with validators,
and requires reviewed notices, release notes and explicit version approval.
Record real-DAW verification or accepted preview limitations before approval.
Publication remains a deliberate review of actual artifacts.

## Consequences

Signing no longer blocks an explicitly unsigned preview. Licensing and host
acceptance still block publication; passing automated checks does not complete
listening, performance or GUI acceptance. The website already recognizes the
unsigned package naming contract. Existing internal candidates are not promoted.
