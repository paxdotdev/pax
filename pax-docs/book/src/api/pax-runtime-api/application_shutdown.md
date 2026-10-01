# application_shutdown
<!-- summary: Asynchronous service cleanup with UI-thread ownership and host wakeups. -->
<!-- tags: api, pax-runtime-api -->

Asynchronous service cleanup with UI-thread ownership and host wakeups.

## Structs
### `ShutdownComplete`
Transferable signal. Dropping it without acknowledgement reports failure.

---

### `ShutdownTicket`
Nonblocking acknowledgement of managed cleanup. A pending ticket retains its
service and all earlier dependencies; a deadline never fabricates completion.
An explicit Err acknowledges quiescence with a cleanup error. Dropping the
signal is not acknowledgement and leaves dependencies retained.

## Traits
### `ManagedService`
A service whose teardown may wait. `begin_shutdown` runs once on the UI
thread and must immediately reject new work, start cleanup without blocking,
and return a ticket. Dependencies registered earlier remain allocated until
all later services acknowledge. Destruction itself must never block the UI.
