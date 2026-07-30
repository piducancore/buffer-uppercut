# ADR 0001: TRUCE is the canonical implementation

- Status: accepted
- Date: 2026-07-30

## Context

Buffer Uppercut was explored through C++/iPlug2, WRAC/WebView, and
TRUCE-native prototypes. None shipped as a compatibility-bound product. The
parallel implementations increased documentation, migration, testing, and
identity complexity without serving existing users.

## Decision

This Rust/TRUCE repository is the sole canonical implementation. Its product
name is **Buffer Uppercut**. C++ and WRAC repositories are archived experiments
and may be consulted for historical ideas only.

The canonical product does not load archived project state, preserve archived
product IDs, exchange archived presets, or import implementation code from
those ports. The existing audio fixture corpus remains as regression evidence
until intentionally superseded; provenance does not create compatibility.

## Consequences

- Product and documentation can optimize for one architecture.
- Legacy migration code and cross-port acceptance work are removed.
- Future contract changes need only account for the canonical product.
- Archived projects must not become hidden dependencies.
- A repository/remote rename may be performed separately from plugin identity.

## Rejected alternatives

- Maintaining three equivalent products: excessive cost and unclear ownership.
- Keeping migration “just in case”: it adds permanent surface area without
  released users.
- Treating the C++ engine as a perpetual oracle: it would prevent the canonical
  Rust product from evolving independently.
