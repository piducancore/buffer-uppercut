# Product website

Astro static website, maintained alongside the plugin. No server, client-side
release API requests, React runtime, or separate `gh-pages` branch is required.

## Local development

Node >=22.12.0 is required.

```sh
cd site
npm ci
npm run dev
npm test
npm run build
npm run test:build
```

The local URL includes `/buffer-uppercut/`. Local builds use an empty release
list by default; this is an honest unpublished state, not sample downloads.
For a live release sync, run `RELEASES_SOURCE=github npm run build`.
An optional `GITHUB_TOKEN` is used only at build time and is never rendered.
API failures fail the production build rather than silently emptying the site.

## GitHub Pages

Repository **Settings → Pages → Source** uses **GitHub Actions**.
The destination is `https://piducan.dev/buffer-uppercut/`, using the account's
existing Pages custom domain. Run the Website workflow to refresh manually.
Build output lives in `site/dist/` and is not committed.

The repository is public, so published release assets can be downloaded without
a GitHub login. Draft releases remain excluded from the website.

The workflow builds pull requests without deploying. Main-branch site and
screenshot changes, published/edited/deleted release events, manual dispatch,
and a daily release sync deploy the website. The daily sync also catches
release events suppressed when another workflow uses `GITHUB_TOKEN`.

## Downloads

GitHub Releases is the source of truth. Drafts never appear. The right column
lists published releases newest first; raw notes are escaped plain text, not
untrusted HTML. Latest downloads select the newest release with eligible
packages; platforms missing from that release remain unavailable rather than
mixing versions. The initial supported filename contract is:

```text
Buffer-Uppercut-VERSION-windows-x64-unsigned.zip
Buffer-Uppercut-VERSION-macos-unsigned.zip
Buffer-Uppercut-VERSION-linux-x64-unsigned.zip
SHA256SUMS
```

Only these explicitly unsigned archives create download buttons. Internal
`not-for-distribution` candidates, standalone packages, and unknown signing
status are excluded. Architecture and runtime requirements must be documented
in release notes. macOS ad-hoc signing is not Developer ID signing.

The unsigned-preview workflow remains draft-only and cannot populate public
downloads until its draft is deliberately published after release review; see
[release inputs](../docs/releases/README.md). The website does not automatically
publish binaries or mark them stable. When trusted signing is introduced, extend the package
metadata, filename recognition, notices and tests together.
