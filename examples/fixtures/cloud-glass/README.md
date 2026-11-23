# Cloud glass

Run the local button comparison from the repository root with the checkout’s Pax CLI.

```sh
cargo run -p pax-cli -- run --libdev --path examples/fixtures/cloud-glass --target web
```

Move the pointer across either face to place a matched scene light on every sample. **Sweep light** parks the light for 2.5 seconds, crosses the face, and returns on an eight-second cycle. **Side by side** cycles through the two isolated treatments. **Light theme** switches the background while keeping the same white and obsidian specimens available for comparison.

The rounded Get Started face follows the website hero’s responsive width and 56 px height. The closed View Source tab measures 112 × 34 px. Its native button opens the WGSL source. The site’s existing frost, obsidian finish, and one-cycle CMY ButtonRim supply the comparison surfaces.

## Tuning the face

```pax
<CloudGlass button_rim=true width=100% height=100% density=2.9 gas_color=WHITE
    gas_opacity=0.95 billow_scale=23.0 phase={self.phase}>
    <Rectangle width=100% height=100% corner_radius=5.333 fill=WHITE/>
</CloudGlass>
```

| Property | Effect |
| --- | --- |
| `density` | Amount of gas, 0–8. Lower values leave fewer, thinner wisps with clear space between them. Zero empties the shell. |
| `gas_color` | Pax color scattered by the gas. The shell reflects the scene light’s own color. |
| `gas_opacity` | Opacity of the gas that is present, 0–1. Lower it to see through the same wisps. Zero leaves the shell and its reflections visible. |
| `billow_scale` | Characteristic billow radius in local logical pixels, 4–128. |
| `phase` | Accumulated drift time in seconds. The host integrates elapsed time at the selected speed, so changing speed preserves the current folds. |

Try a 10 px billow scale and 100% gas opacity. Lower density from 2.6 to 0.8 to see bright tufts separated by clear gaps. With drift paused, lowering gas opacity fades those same tufts in place.

The study opens at density 2.9, opacity 95%, 23 px billows and 4× drift in Cloud white. Both white samples follow the sliders. On the website, View Source tabs use density 3.4 and the fixed-nav Get Started button uses 3.9.

The projected artwork’s alpha defines the glass shape on WebGPU. `button_rim=true` shades the 2 px CMY rim, clear shell and cloud banks in the same pass. Empty space between wisps reveals the page wherever the shell contributes no reflection or bevel.

Keep native foregrounds and hit targets as siblings of the captured face. On the dark study background, the native labels use light ink over sparse gas and a midtone where white wisps alternate with clear gaps. CPU rendering shows the projected artwork directly. This fixture supplies a static tinted gradient for that path.

On web the study follows live changes to the system’s reduced-motion preference and holds both the gas and the light still. The animation clock also stops when the comparison scrolls out of view or its source viewer opens.
