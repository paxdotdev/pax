# pax-web-async
<!-- summary: Scoped browser futures with the same publication/delivery contracts as Pax's -->
<!-- tags: api, pax-web-async -->

Scoped browser futures with the same publication/delivery contracts as Pax's
native adapter. This crate does not provide Tokio networking in browser Wasm.

## Structs
### `BoundTasks`
Local bound executor. Dropping a task control does not detach its lifetime.

---

### `BrowserService`
Managed browser executor service. Futures stay on the browser's thread;
component cancellation aborts their polling and revokes result authority.
