# application_shutdown
<!-- summary: Retained asynchronous resource cleanup with UI-thread ownership and host wakeups. -->
<!-- tags: api, pax-runtime-api -->

Retained asynchronous resource cleanup with UI-thread ownership and host wakeups.

## Structs
### `ShutdownComplete`
Transferable signal. Dropping it without acknowledgement reports failure.

---

### `ShutdownTicket`
Nonblocking acknowledgement of retained cleanup. A pending ticket retains its
owner and all earlier dependencies; a deadline never fabricates completion.
An explicit Err acknowledges quiescence with a cleanup error. Dropping the
signal is not acknowledgement and leaves dependencies retained.
