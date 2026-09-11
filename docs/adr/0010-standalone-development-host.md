# 0010 — Standalone remains a development host

## Status

Accepted.

## Context

TRUCE provides a standalone host with audio-device and MIDI selection, and that
host is useful for fast UI and DSP iteration without a DAW. Buffer Uppercut is
an insert effect, however, and the repository has not defined a standalone
product experience for selecting input, handling absent devices, monitoring,
preventing feedback, capturing system audio, or playing files. Distributing the
executable would create an unsupported product contract before those behaviors
have been designed and accepted.

## Decision

CLAP and VST3 are the supported and distributed product formats. Keep the
standalone feature and executable as a development host. Compile it in CI and
use it for local UI, DSP, direct-key, MIDI, and audio-device testing, but do not
package, sign, advertise, or upload it as a release artifact.

A future decision may promote standalone to a product format only after its
audio-input, monitoring, no-input, packaging, and acceptance behavior are
specified together.

## Consequences

- Developers retain the fast `cargo truce run` workflow.
- CI continues detecting standalone-host compilation regressions.
- Release workflows and user documentation contain CLAP and VST3 only.
- Standalone direct-key defaults remain testable development behavior, not a
  promise to end users.
- Signing and notarization work does not need to cover the standalone executable.

## Rejected alternatives

- Removing standalone entirely would discard a useful development and test host.
- Shipping it with generic device menus would leave core effect-input and
  monitoring behavior undefined.
- Treating successful compilation as product readiness would bypass UX,
  packaging, and real-device acceptance work.
