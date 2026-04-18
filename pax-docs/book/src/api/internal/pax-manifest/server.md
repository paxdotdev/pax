# server
<!-- summary: API docs for pax-manifest::server. -->
<!-- tags: api, pax-manifest -->

## Structs
### `PublishRequest`
Publish request payload used by designer/server flows.

#### Properties
##### `manifest`
Type: [`PaxManifest`](/api/internal/pax-manifest/index.md#paxmanifest)

---

### `PublishResponseSuccess`
Successful publish response payload.

#### Properties
##### `pull_request_url`
Type: `String`

---

### `ResponseError`
Error payload returned by publish flows.

#### Properties
##### `message`
Type: `String`

## Enums
### `PublishResponse`
Publish response union.

#### Variants
##### `Undefined`
##### `Success`([`PublishResponseSuccess`](/api/internal/pax-manifest/server.md#publishresponsesuccess))
##### `Error`([`ResponseError`](/api/internal/pax-manifest/server.md#responseerror))
