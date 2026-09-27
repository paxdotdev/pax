# viewport_proximity
<!-- summary: Opt-in global viewport observation over the runtime's shared prepared geometry. -->
<!-- tags: api, pax-runtime -->

Opt-in global viewport observation over the runtime's shared prepared geometry.

## Structs
### `ViewportProximityStats`
Cumulative observation work, separate from render preparation and application handlers.

#### Properties
##### `index_updates`
Type: `u64`

Indexed geometry entries refreshed after dependency changes.

##### `domain_queries`
Type: `u64`

Scroll domains queried (cold nested domains are pruned).

##### `candidates`
Type: `u64`

Spatial candidates considered, including scroll-owner proxies.

##### `samples`
Type: `u64`

Detailed target samples, including terminal samples for previously active targets.

##### `events`
Type: `u64`

Local event deliveries to registered handlers.
