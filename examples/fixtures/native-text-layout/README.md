# Native text layout fixture

This small native application compares engine layout with real platform text
bounds. It covers omitted width/height, constrained-width wrapping, an
autosized Stacker with a following control, a padded autosized Frame, and two
Text nodes expanded from one repeated template. The repeated nodes must own
independent measurements even when their content and constraints match. The
fixed-height label is a control case: its authored height remains 30px when
system text size increases.

```sh
pax-cli run --path examples/fixtures/native-text-layout --target windows
```

Use **Toggle content** to switch between short and long paragraphs, and
**Toggle width** to switch the paragraphs between 320px and 180px. **Record
layout** writes the current engine IDs, bounds, transforms and measured sizes
to stderr. Packaged desktop cartridges can also append those records to
the file named by `PAX_TEXT_LAYOUT_LOG`; set it before launching the app.
Pair that output with native UI Automation or GTK bounds and glyph diagnostics;
visible text alone does not prove that the engine or its ancestors resized.
The snapshot button also works in baked Release builds.

For accessibility text-size checks, keep display DPI unchanged and test both
fresh launches and changes while the app is running. Restore the original
system setting after testing. Server results do not establish Windows client
or screen-reader qualification.

The [Windows text-layout checker](../../../tests/src/native-platform-probes/windows/README.md#native-text-measurement-qualification)
compares engine bounds, native control bounds, line ranges, and the final glyph
of each paragraph. Its live-change mode first observes the unchanged running
app, before toggling content or width. This distinguishes OS-driven
remeasurement from an incidental layout refresh caused by the test itself.

On October 19, 2026, Debug and baked Release passed cold-launch checks at
100%, 150%, and 200% text size on Windows Server 2022/Tesla T4 at 96 DPI.
Same-process live changes passed in both modes, including upward and downward
changes in Debug. The fixed 30px label clipped at 200% as expected; autosized
text and its parents grew, and each closed process retired all GPU exports.

The [Linux text-layout checker](../../../tests/src/native-platform-probes/linux/README.md#native-text-measurement-qualification)
compares the same engine records with GTK allocations after content growth,
width changes, and content shrinkage. On October 20, 2026, the baseline passed
117 checks on Ubuntu/Tesla T4 with GTK 4.14.5; the identical Debug cartridge
with the earlier host reproduced nine stale-allocation failures.

With `--live-font-dpi --check-text-ink`, Debug and baked Release on GTK 4.14.5
and the optional GTK 4.22.5 Flatpak Release package each passed 357 checks.
Actual XSettings font-DPI changes from 96 to 144 and back resized static text
before the test requested an engine snapshot, in the same process with
unchanged widget scale. Automatic text and its parents grew and shrank, glyph
ink fit its bounds without ellipsis or missing glyphs, and original dimensions
returned after restoring the setting. The fixed-height control retained 30px
and deliberately permits overflow with its default `clip=false`. Every process
closed normally and retired its owned GL publications.
Fresh launches with `--cold-font-dpi 144 --check-text-ink` also passed 222 checks
in each configuration.

The temporary XSettings source refuses to replace an existing settings manager
and removes only its own window on exit. This qualifies X11 settings propagation,
not a desktop's settings interface, Wayland settings portals, other scripts,
or transformed glyph clipping.
