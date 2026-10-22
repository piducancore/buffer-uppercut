# ADR 0015: Expanded Beat Repeat Cell divisions

- Status: accepted
- Date: 2026-10-16

## Context

Beat Repeat needs triplet, dotted, and quintuplet timings beyond the straight
grid. The unreleased product has no saved-state preservation requirement.

## Decision

Give Beat Repeat a separate 15-choice Cell scale, adding 1/3, 1/6, 1/12,
1/24, 3/16, and 1/5 in ascending duration. Use the same table for musical
duration and displayed labels. Keep the other effects' straight grid unchanged.
The exact normalized mapping is specified in CONTRACTS.md.

Update factory and host defaults to preserve intended rhythms. Do not migrate
old host or native normalized values or change parameter IDs or kit layout.

Authorize a test-only translation of v4 Beat Repeat Cell inputs to the new
scale so existing audio references remain useful and unchanged. Apply the same
translation when rendering the historical v4 generator's audio, retaining its
original serialized inputs. Test new capture durations and CELL lookback at
multiple tempos independently.

## Consequences

Existing normalized Cell values can select different durations. Runtime work
remains a bounded table lookup without allocation. Other rhythm controls keep
their existing semantics.

## Alternatives

Expanding the shared grid would also change Reverse, Tape Stop, and Gate beyond
the requested scope. A separate triplet mode would add another control and make
mixed rhythmic automation harder to author.
