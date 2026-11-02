# Neon opacity

Animated nested opacity, moving masks, vector overlaps and genuine native form
controls share one scene. The fixture bundles its Oxanium and Space Mono fonts.

```sh
pax-cli run --path examples/fixtures/neon-opacity --target web
```

Use **PAUSE ANIMATION** in the bottom-right corner to freeze animation time.
Native controls and frame delivery remain active; BEAM, text editing and other
inputs still work. **RESUME ANIMATION** continues from the frozen phase. This
provides an animated/stable-scene comparison without minimizing the window or
stopping the host event loop.

On a native build, opt-in diagnostics can separate live owned storage from
allocation churn:

```sh
PAX_GPU_MEMORY_DIAGNOSTICS=1 PAX_PROPERTY_MEMORY_DIAGNOSTICS=1 \
  pax-cli run --path examples/fixtures/neon-opacity --target linux
```

Set the same environment variables before launching a Windows executable.
`PaxGpuMemory` reports the first and every 120 encoded frames for each renderer:
retained capture/scratch/alpha texture counts and logical bytes, cumulative
allocations/replacements, buffer capacities and pending commands/screenshots.
`PaxGpuCacheMemory` reports the first and every 120 flushed turns of each logical
layer, in both Debug and Release. It includes cumulative vector-resource churn
and the largest cache-byte observations reported during the interval and over
that layer target's lifetime. These are observed peaks: the existing cache
statistics do not supply a current gauge on turns without cache operations, so
a zero interval does not mean an empty cache. Shared tile caches are counted
once per logical layer, rather than summed across sibling tiles.
`PaxPropertyMemory` reports the first and every 120 property imports: active and
dormant attachments, binding scopes, queued notifications, and local property
count. All these diagnostics are off by default and unavailable on web.

These diagnostics do not include driver residency, deferred GPU frees, reusable
staging-belt storage, allocator retention or native toolkit storage.
Context-wide pipeline counts repeat across
renderers sharing that context; local property counts repeat across graphs on
the same thread. Do not sum those repeated values. Compare process memory and
owned storage against both elapsed time and encoded frames, because animation
advances once per application tick. Host callbacks and publications are not
displayed frame-rate measurements.

The accepted fractional native/vector punch-through approximation still applies;
this fixture does not establish exact translucent color composition.
