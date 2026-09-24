# Local `truce-core` backport

Published TRUCE 6.3.0 with an additive persistent-state notification API:
`EditorBridge::mark_state_dirty` defaults to no-op; `PluginContext` forwards it
and offers a callback decorator for format wrappers. Existing `ClosureBridge`
constructors and standalone wrappers remain source compatible. No product
mapping or serialization logic belongs here.

Remove this exact-version override after an equivalent upstream API is used.
