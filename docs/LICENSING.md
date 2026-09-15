# Licensing and distribution

Original Buffer Uppercut code, including the DSP, kit codec and website source,
is licensed under the root [MIT license](../LICENSE). Package metadata reflects
this choice. Vendored patches retain their upstream licensing; a root license
does not replace dependency licenses or grant rights to upstream trademarks.

## Dependency obligations

- TRUCE 6.3 uses `LicenseRef-TruceLicense-1.0` for framework crates. Its
  [license](https://github.com/truce-audio/truce/blob/main/LICENSE) expressly
  permits shipping audio plugins under its permissive grant. Preserve the
  applicable upstream notices and license texts, including the framework rider;
  do not label the dependency itself as simply MIT.
- Slint 1.15.1 is alternatively licensed under GPL-3.0-only,
  LicenseRef-Slint-Royalty-free-2.0, or commercial terms. Distribution of this
  desktop plugin uses the royalty-free option, not GPL. Preserve its license
  text and display the official Slint attribution badge on the download page,
  as permitted by section 2(b) of the packaged royalty-free license.
- Transitive Rust libraries, embedded fonts and native components may impose
  additional notice or redistribution requirements. Review the actual selected
  versions and bundled resources, not only direct dependencies or latest
  upstream metadata.

## Release gate

Each public ZIP must include `LICENSE`, installation instructions and reviewed
third-party notices/license texts appropriate to the bundled components.
Complete the dependency and resource notice review before publishing a preview.
Automated license discovery is assistance, not evidence that every distribution
condition has been satisfied.

The MIT choice does not waive the outstanding DAW, CPU or release acceptance
checks in [Testing](TESTING.md) and [Roadmap](ROADMAP.md).
