# Local `truce-vst3` backport

Published TRUCE 6.3.0 with persistent editor-state dirty notification through
the additive core API. The shim schedules a coalesced atomic notification using
its existing main-thread restart scheduler, queries optional
`IComponentHandler2`, calls `setDirty(true)`, and releases that interface.
The dirty drain additionally checks the component-creation thread identity,
so editor-owned threads entering existing edit callbacks leave it queued.
No fake parameter gesture or restart is emitted. The ABI matches the official
[Steinberg interface](https://github.com/steinbergmedia/vst3_pluginterfaces/blob/master/vst/ivsteditcontroller.h).

macOS uses the main dispatch queue; Windows uses the existing host-thread timer;
Linux uses the host run-loop timer while an editor is attached, with existing
host parameter polling callbacks as fallback. Hosts omitting the optional
interface cannot receive this notification; explicit state save still includes
the metadata. Host behavior requires real-DAW acceptance.

Remove this exact-version override once equivalent behavior ships upstream.
