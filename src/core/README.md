# Core runtime

The core runtime owns durable raw capture, run storage and recovery, local analytics,
configuration, filtering, process status preservation, and shared display helpers.

Product paths are centralized in `src/product.rs`. ContextDroid has no remote telemetry module.
Project-local custom filters are inherited compatibility functionality and remain trust-gated;
they are not automatically selected for Android safe-profile commands.
