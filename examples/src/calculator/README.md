# PAX-83+ Silver Edition graphing calculator

A silver handheld calculator built from Pax vectors, native Text, and a
two-dimensional Scroller. Calculate and Graph share a small Rust expression
engine. This calculator language is application logic, not PAXEL; it does not
execute arbitrary Rust or JavaScript.

The opening screen is a polar rosette, `r=1+3*cos(11*θ)`: eleven outer petals
and eleven nested inner petals, closed over 0…2π. Its outer radius is 4 and
inner lobes reach radius 2. The opening view chooses the largest existing zoom
step that fits the whole flower with a margin, including when native safe-area
insets settle. It refits on resize until the first keypad, editor, or graph
interaction; afterward resizing preserves the user's chosen scale and center.
CALCULATE opens an empty calculation history, and Cartesian mode retains `sin(x)`.

From the monorepo root:

```sh
cargo build -p pax-cli
PAX_WORKSPACE_ROOT="$PWD" target/debug/pax-cli run --path examples/src/calculator --target web --hot-reload=all --libdev
```

Use the CLI built from this checkout when developing unreleased capabilities.
Native builds use `--target macos`, `ios`, or `ipados` and require the normal
Xcode setup; see the verification limits below. On this workstation, source `~/.zshrc` and
run `nvm use` before commands that build the web interface.

## Controls

Click/tap the keys or use a hardware keyboard. Clicking an expression places
its character cursor. Function keys insert the opening parenthesis too.

| Input | Action |
| --- | --- |
| Enter or `=` | Evaluate in Calculate; repeat with `ans` when the prompt is empty; submit the function in Graph |
| Left / Right | Move the insertion point; pan when the graph has focus |
| Up / Down | Recall Calculate history; pan when the graph has focus |
| Backspace / Delete | Remove the previous / following character |
| Home / End | Move to the start / end of the expression while editing |
| Clear key | Clear the active expression and error |
| Tab | Switch between Calculate and Graph |
| Escape | Finish graph editing, or dismiss a Calculate error |
| GRAPH | Switch to Graph; while already graphing, submit the expression; center the origin and focus the editor |
| Zoom + / − | Enlarge / shrink Calculate text; double / halve Graph magnification |
| 2nd | Arm the next key’s secondary function; press again to cancel |
| 2nd → x θ (MODE) | Toggle Cartesian/polar in Graph; no action in Calculate |
| x θ | Insert the active graph variable; no action in Calculate |

Calculate keeps the latest 20 successful expressions and answers in a scrollable
LCD history. Scroll or swipe above the fixed input area to read older entries.
New results reveal the newest entry; scrolling never changes the input draft.
Zoom and window resizing reflow history, keeping the bottom pinned when already
there and approximately preserving the scroll position otherwise.

Enter adds a history entry and clears the prompt. An operator immediately
after an answer starts an expression with `ans`; other input starts fresh.
Up recalls an expression for editing; Down past the newest restores the draft.
Clear does not erase history or `ans`. The modes preserve separate drafts.
Reloading or rebuilding application logic starts a new session.

Enter on an empty Calculate prompt repeats the last successful operation with
the current answer: `7×6` followed by three Enter presses gives 42, 252, then
1512. The history shows the repeated formula as `ans*(6)`. For an expression
without `ans`, repetition replaces the left operand of the outermost binary
operation, preserving its right operand and grouping: `2+3*4` repeats as
`ans+(3*4)`. A function such as `sqrt(256)` repeats as `sqrt(ans)`, unary minus
repeats as `-ans`, and a lone number/constant repeats as `ans`.
If the formula already contains `ans`, it repeats unchanged using the latest
answer everywhere: `ans*2+1` keeps applying that whole formula. Clear and mode
switches retain the repeat operation; a new successful calculation replaces it.
A repeated calculation that fails leaves its formula editable and preserves
the last valid answer. Before the first successful calculation, empty Enter
is a no-op.

The calculator starts in polar Graph mode. Edit its formula, then press Enter or GRAPH. Until a valid
submission, the last plotted function remains visible. Errors retain the
draft, move the cursor to the problem, and report its character column.
Drag with a mouse, scroll with a trackpad, or swipe the graph with touch.
The D-pad and arrow keys pan once the plot has focus. There is one visible curve. `2nd` → `x θ` (MODE) switches between Cartesian `y=f(x)` and
polar `r=f(θ)`, preserving each formula draft and last valid plot separately.
The `x θ` key inserts the active variable and does nothing in Calculate.
Polar mode starts with `1+3*cos(11*θ)` and samples **0 ≤ θ ≤ 2π** in radians;
negative radii are supported. The angular interval is currently fixed, so
spirals and nonperiodic formulas display only that single revolution.
Zoom has seven levels from 0.25× to 16×; the LCD shows the current level.
Zooming retains the draft without submitting it. GRAPH submits when already
in Graph mode, then centers and focuses the expression. A failed submission
retains the last valid curve and shows its error inside the LCD.
The secondary `x²` and `1/x` functions insert `^2` and `^(-1)` after the current operand,
or apply to `ans` immediately after a successful calculation.
Keys always retain their opaque face and press feedback. `x θ` and its secondary MODE do nothing in
Calculate, and zoom keys do nothing at their respective limits.

The amber `2nd` key arms one subsequent keypad press. The LCD shows `2ND`
and secondary legends change color while armed. Press `2nd` again or Escape
to cancel; a key without a secondary function performs its primary action and
clears the latch. Hardware typing remains literal and also clears the latch.

| Primary key | Secondary function |
| --- | --- |
| sin / cos / tan | asin / acos / atan (inverse trigonometry, radians) |
| ln / log | eˣ / 10ˣ (insert `e^(` / `10^(`) |
| √ | x² (append `^2`) |
| xʸ | 1/x (append `^(-1)`) |
| π | e |
| x θ | MODE (Cartesian/polar) |

`sec` and `csc` remain available when typed, but no longer occupy keys.

## Expression language

| Form | Meaning / example |
| --- | --- |
| Numbers | `12`, `.5`, `2.`, `1e-3` |
| Arithmetic | `+`, `-` / `−`, `*` / `×`, `/` / `÷`, `^` |
| Grouping and sign | `(2+3)*4`, `-2^2` → `-4`, `2^-3` → `0.125` |
| Powers | Right associative: `2^3^2` → `512` |
| Constants | `pi` / `π`, `e`, and the latest successful Calculate `ans` |
| Functions | `sin(...)`, `cos(...)`, `tan(...)`, `sec(...)`, `csc(...)`, `asin(...)`, `acos(...)`, `atan(...)`, `sqrt(...)` / `√(...)`, `ln(...)`, `log(...)` / `log10(...)` |
| Variable | Cartesian: `x`; polar: `θ` or `theta`. Only the active Graph variable is accepted. |

All trigonometry uses **radians**. The previous basic calculator used degrees;
convert degree inputs explicitly, for example `sin(30*pi/180)`. Function names
are case insensitive. Parentheses and explicit multiplication are required:
write `2*pi` and `2*sin(x)`. `sec` and `csc` are reciprocal cosine and sine;
`asin`, `acos`, and `atan` are inverse trigonometric functions and return
radians. `asin` and `acos` require inputs in −1…1. `ln` is natural log; `log` is
base 10.
Scientific notation requires an exponent after `e`, so use `2*e` for twice
Euler's constant. Graph snapshots `ans` when submitted; later calculations
do not silently change the plotted function.

Values use real `f64` arithmetic. Division by zero, invalid roots/logarithms,
non-real powers, poles of tangent/secant/cosecant, and overflow report errors. Results
show up to 12 significant digits; subsequent calculations retain the full
stored value. There is no symbolic simplification, complex arithmetic,
implicit multiplication, or support for user-defined functions.

## Responsive display and graph limits

The LCD defaults to 16-pixel monospace text, 9.6-pixel character cells, and
22-pixel rows. Calculate zoom selects 12, 14, 16, 20, or 24-pixel text; cell
width and row height scale with it, changing how much history fits. Graph
keeps 16-pixel formula text and its own independent plot zoom. A larger window
adds columns and rows at the chosen text size. History reflows and the editor
follows its cursor, reserving at least one history row on a short LCD. The body grows up to 1,280
pixels wide and the screen up to 1,000 pixels tall. At 320-pixel phone width,
key touch areas remain at least 44 × 44 pixels. The three-column upper bank
contains 2nd/zoom, trig, and log/root rows; the D-pad centers over the final
two keypad columns and has no center key. Secondary functions reduce the
main keypad to five rows. Each key has a 31.5-pixel face below a 9.2-pixel secondary
legend, inside a 47-pixel touch area, with a 51-pixel row pitch. The bevel depth
and press travel are 40% of the previous design. CALCULATE and GRAPH use
approximately half-height faces with smaller type, retaining 44-pixel hit
areas. The helper-text row is removed and errors appear inside the LCD.
The layout reserves the keypad and its gutters before assigning remaining height to the LCD.
Bottom clearance matches the visible side clearance, accounting for the
key bevel and the case's thicker bottom rim.
The graph editor uses one line that follows its cursor. Short windows scroll the whole calculator.

Native iOS and iPadOS use a viewport-width silver surface with no artificial
outer rim, shadow, or rounded corners; the device supplies the outer outline.
Web and macOS use the same edge-to-edge surface below 600 logical pixels of
window width, and retain the freestanding case at larger widths. Four explicit
`DynamicIslandSpacer` components bind the live top, right, bottom, and left
insets. The content viewport reserves those edges for the status bar, island,
and home indicator, including after rotation. The silver background remains
fixed and fills the whole window. On a tall native viewport the LCD grows to
use the available height; short viewports scroll while preserving key sizes.
The spacers contribute zero inset on web and macOS.

Graph magnification 1× is 16 logical pixels per unit on both axes. Zoom ranges
from 4 to 256 pixels per unit. On resize it keeps the chosen scale, redraws
for the larger viewport, preserves its center where the finite boundary
permits, and shows more of the function. The native Scroller covers world
coordinates −256 through 256 on each axis; its pixel extent grows with zoom
(from 2,048 to 131,072 pixels square). Geometry is sampled only for the
visible viewport, with no overscan margin, and refreshed after scroll movement. Scroll offsets are the source
of truth for both axes.

Input is limited to 256 characters and parser nesting to 32. Each plot refresh
is bounded to 4,096 output points and 250,000 expression-node visits. Interval
checks and adaptive refinement leave gaps across poles and undefined domains.
Each initial screen interval receives a fair share of the remaining work and
geometry budget, so dense detail cannot consume the right side's allocation.
Off-screen bounds are discarded before point sampling. At pixel resolution,
unresolved continuous Cartesian detail is shown as a conservative vertical
range envelope; `PIXEL ENVELOPE` identifies this approximation. It can include
values within an interval's enclosure that the curve does not actually attain.
Polar intervals use projected Cartesian bounds for culling; only narrow bounds
can become envelopes. Polar chord acceptance measures screen-space geometric
deviation, enclosing the whole interval in coordinates aligned with the chord.
Smaller range checks tighten that enclosure without adding drawing vertices;
uneven speed along a nearly straight stroke does not require extra segments.
A bounded second pass shares unused budget with unresolved angular slices.
The work and output caps remain unchanged. `DETAIL LIMIT` indicates remaining unresolved detail,
including overly complex expressions or broad unresolved polar intervals.
The plot is not a proof that every root or singularity has been found.

For example, `4*cos(3*θ^2)` has a wide first sweep: at θ=0 the radius is 4,
and its first zero is θ=√(π/6), about 41.46°. Subsequent sweeps narrow because
the cosine's phase grows quadratically. Negative radii lie on the opposite ray.
This formula is not 2π-periodic, so its endpoints do not meet; the plot never
adds an artificial closing segment.
`NO RESOLVED CURVE` can mean an empty real domain, a curve outside the viewport,
or an expression whose bounds could not establish continuous segments.
`WORLD EDGE` identifies the finite scrolling boundary.

This is a calculator-owned editor: native selection, paste, IME, and automatic
screen-reader button semantics are not implemented. Web hardware-key input
requires page/body focus. Text uses local Courier-family fonts; rasterization
can vary between platforms. WGPU provides a broad pointer-following highlight on the chassis; Piet keeps
the authored colors and bevels without lighting. The LCD stays unlit.

Earlier debug and baked release builds were visually checked on the iPhone
16 Pro Max simulator (iOS 18.2), in portrait and landscape, including safe-area
clearance and initial graph positioning. Native tap routing has one activation
path through the touch observer. Scroller presentation offsets follow authored
position changes so the initial plot, recentering, and zoom reach the renderer.
An earlier macOS observation of a blank initial window has not been retested.

The previous physical iPhone zoom-out crash was a separate native surface
allocation bug: a 4,096-point pane became a 12,288-pixel texture at 3x density.
Native tiling now accounts for the actual screen density. Runtime regressions
cover allocation bounds and viewport coverage at 1x, 2x, and 3x, and a scripted
iOS release check completed zoom-out to 0.25x and back through 2x. The dense
curve cutoff fixed here was the application sampler's work allocation; it
required no change to the GPU budget or tiling policy.

For this iteration, the core suite passes in debug and optimized builds,
including dense-curve coverage, poles, enormous finite slopes, polar circles
and roses, mode/draft preservation, second-function latching, inverse trig, and editing. Phone-width web interaction
checks cover polar plotting, `sin(x^2)` after zoom-out, second-function
calculation, GRAPH submission, and visible error recovery. Physical iOS gestures
and appearance still need a hands-on device pass; web checks do not establish
native interaction correctness. Pinch zoom is not implemented yet.

## Checks

The parser, editor, responsive geometry, and plotting tests can run without
building a graphical chassis:

```sh
rustc --edition=2021 --test examples/src/calculator/tests/core.rs -o /tmp/pax-calculator-tests
/tmp/pax-calculator-tests
```

`src/lib.pax` owns the visual composition; `key.pax` owns the tactile key.
`model.rs` owns editing/history, `expression.rs` parses and evaluates,
`graph.rs` bounds curve generation, and `layout.rs` derives responsive sizes.
