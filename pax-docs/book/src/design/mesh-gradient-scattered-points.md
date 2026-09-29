# PAX-1011 — Scattered-point interpolation investigation

September 29, 2026. Design research requested by Zack after reviewing the
[original grid proposal](mesh-gradients.md). No paint API, backend extension,
or choice of interpolation model has been approved. The original proposal is
preserved as the exact transferred snapshot; this note records the subsequent
investigation.

## Finding and recommendation

Independent, unordered colored anchors are a credible option. Inferring
neighbors does not force the color jumps seen with linear triangle interpolation.
The best candidate from this investigation is **Sibson natural-neighbor C1
interpolation with zero color derivatives at anchors**. It combines exact
anchor values, local influence, smooth spatial color, and bounded color/alpha
without a user-authored grid. The zero-derivative choice creates soft plateaus
around anchors; that is an explicit visual policy to review.

Recommend a bounded GPU feasibility experiment for this candidate before choosing
the public API. Keep inverse-square Shepard interpolation as an explicit cheaper
comparison, not an automatic fallback. Do not choose an unconstrained thin-plate
spline as the default for saturated animated colors: the probe found significant
overshoot, aggravated by nearby anchors.

These are **scattered color fields**, not deformable bicubic patch meshes.
A future API for Bézier patch edges would be a separate mesh model, not simply
extra handles on this field. The issue's original genuine-mesh requirement must
be reconciled with that distinction before implementation. This research does
not approve renaming a field interpolator to a patch mesh.

## Models examined

| Model | Anchor behavior and appearance | Main limitation | Evidence |
| --- | --- | --- | --- |
| Linear Delaunay triangles | Exact values; continuous across edges | Color slope creases; connection flips can jump | Four-point numerical control case |
| Ordinary Sibson natural neighbors | Exact, local, bounded; reproduces affine fields | Not generally differentiable at anchors | Implemented and checked |
| Sibson C1 with zero anchor derivatives | Exact, local, bounded; soft flat regions around anchors | More work per sample; gives up affine reproduction | Implemented and checked |
| Inverse-square Shepard | Exact, bounded; no topology or linear solve | Global radial influence; flat spots and less local control | Implemented and checked |
| Thin-plate spline, affine polynomial tail | Exact, smooth global field | Overshoot, global influence, conditioning near collisions | Implemented and checked |
| Clough–Tocher cubic triangles | Exact, spatially C1 triangle patches | Triangulation changes still require motion investigation | Documentation review only |

Natural neighbors use the area a query's Voronoi cell takes from each anchor's
cell as normalized weights. Ordinary Sibson interpolation reproduces the data
and affine fields; its spatial derivatives need not agree at anchors. A C1
extension adds specified or estimated derivatives. The reference here supplies
zero derivatives, rather than introducing gradient-fitting behavior.
[CGAL definitions and C1 formula](https://doc.cgal.org/latest/Interpolation/index.html).

With that choice, the C1 formula becomes a positive mixture of two normalized
weighted color sums. Therefore it stays in the convex hull of the supplied
colors. This also preserves valid premultiplied RGBA: if each input satisfies
`0 <= RGB <= alpha <= 1`, so does the output, apart from rounding. This is an
algebraic consequence of the chosen formula, not a guarantee for C1 variants
with arbitrary fitted derivatives.

The inverse-square reference uses all anchors with normalized `1 / distance²`
weights and the exact-anchor limiting value. It has zero first derivatives at
distinct anchors. It avoids hard neighbor selection; restricting it to the
nearest few anchors would be a different algorithm needing continuity work.
Distance-based and Voronoi-based influence have different locality and visual
characteristics. [Park et al., Discrete Sibson Interpolation](https://escholarship.org/content/qt88c9892g/qt88c9892g.pdf).

The thin-plate reference uses `r² log(r)` plus an affine polynomial, with exact
interpolation and no smoothing penalty. Its coefficients come from a linear
system; coincident inputs or an insufficient polynomial rank are problematic.
Smoothing can change the conditioning but relaxes exact interpolation.
[SciPy's RBF formulation](https://docs.scipy.org/doc/scipy/reference/generated/scipy.interpolate.RBFInterpolator.html).

Clough–Tocher constructs cubic patches over a Delaunay triangulation and estimates
derivatives to reduce curvature. Its documented spatial C1 property does not
alone establish continuity as the input positions and triangulation change.
It was not implemented or rejected by a numerical test in this investigation.
[SciPy's Clough–Tocher documentation](https://docs.scipy.org/doc/scipy/reference/generated/scipy.interpolate.CloughTocher2DInterpolator.html).

## Numerical evidence

The independent Python reference uses double precision, direct polygon clipping
for Voronoi areas, and NumPy's dense linear solve for the thin-plate spline.
No SciPy, CGAL, Pax renderer, or GPU implementation is used by the probe.

The color fixture has seven anchors: four rectangle corners and three irregular
interior locations. Every anchor has an independent position. All four methods
were evaluated at the same 144 × 144 sample centers, using encoded-sRGB channel
values. Images clamp thin-plate output to the displayable range; measurements
use the raw values.

Checks and observations:

- All four methods reproduced anchor colors with maximum channel error below
  `9e-16`. The direct-at-anchor branch is supplemented by nearby samples below.
- Ordinary natural-neighbor weights reproduced affine positions to `4.4e-13`
  over 128 seeded interior samples. Both natural-neighbor weight sets were
  nonnegative and summed to one within `2.3e-16`.
- Reordering anchors changed sampled results by less than `3.2e-13`; authoring
  order is not a paint-order policy for these interpolants.
- Natural-neighbor methods and Shepard stayed in range. The thin-plate fixture
  reached approximately `-0.127` to `1.167`; 46.6% of sampled pixels had at least
  one RGB channel outside [0,1]. This is fixture-specific, not a universal rate.
- A seeded translucent-color check stayed within premultiplied bounds for all
  four methods. It is not evidence that thin-plate interpolation is bounded;
  the saturated fixture already supplies a counterexample.

### Movement through an edge flip

Four square-corner anchors alternate two colors. The upper-right point moves
from y = -epsilon to y = +epsilon. At three fixed interior sample locations,
the maximum absolute channel change was:

| epsilon | Linear triangles | Natural neighbors | C1, flat anchors | Shepard | Thin plate |
| --- | ---: | ---: | ---: | ---: | ---: |
| 0.01 | 0.995 | 0.00995 | 0.01124 | 0.00789 | 0.01161 |
| 0.0001 | 0.99995 | 0.000100 | 0.000113 | 0.000079 | 0.000116 |
| 0.000001 | 0.9999995 | 0.00000100 | 0.00000113 | 0.00000079 | 0.00000116 |

The four scattered interpolants' differences shrink with the perturbation;
the linear-triangle control approaches a full-channel jump. This demonstrates
that a connection change need not produce a color jump. It is one motion
experiment, not a proof for every trajectory, boundary event, or collision.

### Smoothness at an anchor

At the interior yellow anchor, reducing a horizontal finite-difference step
from 0.01 to 0.0001 left ordinary natural neighbors' opposing slope mismatch
near 7.16 channel units per normalized coordinate. The C1 flat-anchor mismatch
fell from 0.367 to 0.00370; both one-sided slopes approached zero. Shepard
showed the same zero-slope tendency. This makes the plateau versus pointed
anchor distinction visible in the comparison image.

### Approaching differently colored anchors

For a separate scalar fixture with two central values of 0 and 1 and four
corners at 0.5, the thin-plate field became increasingly extreme:

| Central separation | Raw sampled range | Matrix condition number |
| --- | --- | ---: |
| 0.1 | -0.47 to 1.47 | 138 |
| 0.01 | -4.35 to 5.35 | 6,807 |
| 0.001 | -31.74 to 32.73 | 452,486 |

Clamping conceals numerical range excursions but makes large saturated regions.
The bounded methods avoid overshoot, yet must still make a steep transition
between very close, different colors. Coincident anchors with conflicting
values cannot both be interpolated exactly by a single-valued field.

## Renderer implications and cost

These are algorithmic estimates, not measured GPU performance:

| Candidate | When positions change | Sampling cost | Color-only changes |
| --- | --- | --- | --- |
| Shepard | Upload positions | O(N) distances and weighted sums per sample | Reuse geometry; update colors |
| Thin plate | Dense factorization, generally O(N³) | O(N) kernel evaluations per sample | Reuse factorization; solve new color coefficients |
| Natural neighbors / C1 | Update geometric neighborhood representation | Variable local polygon/area work, then weighted sums | Reuse geometry; update colors |
| Original bicubic grid | Recompute local patch coefficients | Bounded patch evaluation through tessellation | Keep topology; update color coefficients |

Direct global sampling at 1024 × 1024 with 16 anchors performs about 16.8
million anchor/sample interactions per field raster. That arithmetic count is
not a GPU timing estimate. Natural neighbors need more geometric work than a
single distance term. Their locality helps, but does not guarantee cheap shader
execution or bounded register usage.

GPU-oriented discrete Sibson methods exist, but they approximate the continuous
field. Resolution error, anchor accuracy, and motion under subpixel changes
would need separate validation; an approximate algorithm must not inherit the
exact reference's guarantees without evidence.
[Park et al.'s GPU-oriented method](https://escholarship.org/content/qt88c9892g/qt88c9892g.pdf).

The Python reference took approximately four seconds for the four 144 × 144
fields together on this workstation. Its interpreted clipping loops are a
correctness aid, not a rendering architecture or a predictor of compiled/GPU
speed. No production frame-rate claim is supported by this result.

The current Pax geometry shader has a gradient storage binding and a separate
alpha-mask sampler. Its `GpuGradient` remains a 256-byte, eight-stop record.
Scattered anchors need a deliberate new data path; the alpha mask binding is
not spare paint storage. A simple direct sampler could avoid a paint-texture
atlas but would repeat field work across consumers and captures. Retained paint
textures remain a plausible shared solution; this research does not settle
that architecture gate.

## Authoring semantics still to decide

1. **Field versus patch mesh.** A field could take one flat array of
   `{position, color}` records, with no dimensions or row inference. A grid
   remains useful for persistent edge connections, folds, and future Bézier
   handles. Do not promise those mesh controls for a topology-free field.
2. **Domain.** Natural neighbors are defined over the convex hull of the
   anchors. Four explicit corner anchors can cover a rectangle without imposing
   rows on interior points. Transparent outside, extension from the hull, and
   automatic boundary anchors are different policies; no implicit extra
   anchors are recommended. A convex hull alone cannot encode a concave hole.
3. **Distance metric.** Interpolation in normalized paint coordinates preserves
   a composition under aspect-ratio changes. Euclidean logical-pixel distances
   instead change influence as the box resizes. Both can accept Pax Size values;
   resolve those against stable bounds first, then apply the chosen metric.
   Ordinary element transforms should carry the resulting field with geometry.
4. **Collision/degeneracy.** Conflicting coincident points, all-collinear points,
   nonfinite values, minimum separation near floating-point limits, and maximum
   counts need explicit diagnostics and semantics. Do not silently sort into
   rows, merge points, or substitute another gradient. Spatial C1 does not make
   arbitrary point-collision animation continuous.
5. **Smoothing.** Zero anchor derivatives are a useful bounded default candidate.
   Fitted derivatives may give a different look and can lose the convex-color
   guarantee. Neither should be an undocumented quality-level substitution.

Point/color bindings, whole-Paint crossfades, stable full-path paint bounds,
materials, masks, opacity, and debug/baked-release behavior must retain the
contracts from the original proposal. One correction to that snapshot:
Handwriter currently produces one combined Path, so its direct stroke paint
domain covers the complete generated path, not individual glyph paths.

## Recommended next experiment

Before selecting public syntax, compare bounded GPU implementations of the C1
natural-neighbor candidate and Shepard against the double-precision reference.
Use 7, 16, and 64 anchors as test sizes, not promised API limits. Exercise
ordinary motion, near collisions, hull changes, alpha, extreme aspect ratios,
and color-only updates at desktop/mobile paint sizes and DPR 1/2. Measure both
single and repeated consumers. Establish approximation error and memory limits
alongside CPU/GPU work. Keep this isolated from Pax's mounted-node lifecycle and
consumer tessellation.

No parser, Paint enum, binary format, renderer, website, or canonical public
capability documentation changed in this investigation. The ticket remains
In Progress. A production choice still needs Zack's review.

## Reproducing the probe

Source: `pax-docs/research/pax-1011/scattered_points.py`. Requires Python, NumPy,
and Pillow; the run used NumPy 2.3.5 and Pillow 12.3.0. From the repository root:

```sh
python3 pax-docs/research/pax-1011/scattered_points.py --output-dir /tmp/pax-1011-study
```

Use a Python environment containing those packages. The output directory receives
the comparison image, individual field images, and a JSON record of the checks.
Generated outputs belong outside the repository. The reference is purpose-built
for these probes: its finite Voronoi clipping box detects an insufficient extent
and fails rather than claiming to implement a general robust geometry library.
