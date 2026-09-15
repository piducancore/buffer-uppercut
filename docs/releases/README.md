# Unsigned preview release inputs

Before creating `vVERSION`, prepare and review:

1. Root Cargo version `VERSION`, including `-preview.N`; update the lockfile.
2. `VERSION.md` with exact platform architectures, OS/runtime requirements,
   source revision, installation/security limitations and known issues.
3. `THIRDPARTY.json`, generated with cargo-bundle-licenses 4.2.0 and manually
   reviewed. Resolve all missing/custom license texts, preserve required
   copyrights/notices and inspect embedded-font/native resource licensing.
4. Record real-DAW smoke tests for every advertised platform and format,
   tied to the exact candidate artifacts. Complete the relevant checks in
   `docs/TESTING.md`, or explicitly document accepted preview limitations.
5. After review, add the exact `vVERSION` as a line in `APPROVED.txt`.

Neither the notice baseline nor approval file exists until actually reviewed;
the release workflow deliberately fails closed when they are absent. Do not add
placeholder approvals to make CI green. The current stable-shaped crate version
is not accepted by the unsigned-preview workflow.

License discovery:

```sh
cargo install cargo-bundle-licenses --version 4.2.0 --locked
cargo bundle-licenses --format json --output target/THIRDPARTY-audit.json
node tools/check-notices.mjs target/THIRDPARTY-audit.json
```

Copy reviewed license texts into the baseline using `apply_patch`. The generator
includes a conservative dependency set, including development-host/build-only
dependencies. A generated baseline is not itself a legal compliance review.

The workflow validates plugin bundles before staging, compares dependency
notices to the baseline and creates a draft prerelease with unsigned CLAP/VST3
ZIPs, LICENSE, INSTALL.txt, release notes and SHA256SUMS. It never publishes
automatically and refuses to overwrite a published release. Review the actual
draft artifacts before publishing; publication and a Website manual dispatch
then make the downloads available. Signing and stable promotion are separate.

Preview 1 targeted macOS and Windows based on recorded Ableton Live use.
Preview 2 adds an x86-64 Linux package built on Ubuntu 22.04. Linux currently
has automated CLAP/pluginval coverage only and must be described as unverified
in a real Linux DAW. Do not claim LMMS compatibility: current LMMS releases do
not natively host CLAP or VST3.
