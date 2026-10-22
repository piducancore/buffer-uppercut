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

## Commit, publish, and refresh the website

After the release inputs above are reviewed, update all three product crate
versions and their lockfile entries. The DSP/kit entries in the notice baseline
also carry those versions; compare a fresh license inventory before approval.
Stage the specific release files, not generated analysis folders or build output.

Example for preview 5 (substitute the next version for future releases):

```sh
git diff --cached --check
git commit -m "Release preview 5 with expanded Beat Repeat cells"
git push origin main
git tag -a v0.1.0-preview.5 -m "Buffer Uppercut 0.1.0-preview.5"
git push origin v0.1.0-preview.5
gh run list --workflow release.yml --limit 5
gh run view RUN_ID
```

The tag starts **Unsigned preview candidate**. Wait for every job to succeed.
If the workflow needs a deliberate retry, use
`gh workflow run release.yml -f tag=v0.1.0-preview.5`; do not move a published tag.
Inspect the draft notes and download its three platform ZIPs plus `SHA256SUMS`
into a fresh temporary directory. Verify checksums, CLAP/VST3 bundle contents,
installation instructions, licenses and release-note versions before publishing.
Do not bypass failed checks or overwrite published assets.

```sh
gh release view v0.1.0-preview.5 --json isDraft,assets,body
gh release download v0.1.0-preview.5 --dir REVIEW_DIRECTORY
# From REVIEW_DIRECTORY on macOS:
shasum -a 256 -c SHA256SUMS
gh release edit v0.1.0-preview.5 --draft=false --prerelease
gh workflow run pages.yml --ref main
gh run list --workflow pages.yml --limit 5
```

Wait for the Website deployment to succeed, then verify that
https://piducan.dev/buffer-uppercut/ shows the new version and all three download
links. The website reads published GitHub Releases at build time; no manual
download-URL edit is required. Publishing binaries and refreshing Pages are
separate steps. See [website behavior](../../site/README.md).
