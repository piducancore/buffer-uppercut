# 0011 — Static product website alongside the plugin

## Status

Accepted.

## Decision

Maintain an Astro static website in `site/` in the canonical repository. Develop
on temporary feature branches and merge into `main`; do not maintain a permanent
website source branch. GitHub Actions deploys generated output to GitHub Pages.
GitHub Releases hosts binaries and provides release history at site build time.

The download page uses platform cards and a right-hand release-history column,
stacked on mobile. Publish no invented versions, draft releases or internal
candidate downloads. Unsigned distributable packages are explicitly labeled;
stability and trusted signing are independent properties. Standalone remains
excluded under ADR 0010. This decision does not publish or promote binaries.

## Consequences

Website content and committed editor screenshots evolve alongside the plugin.
No runtime backend or per-visitor GitHub API access is needed. Production API
sync failures preserve the previous deployed site by failing the new build.
Rebuilds refresh release history; a daily sync covers suppressed workflow events.
Signing can be added later without changing hosting or the page layout.
