# PAX-83+ Silver Edition graphing calculator

A silver handheld calculator built from Pax vectors, native Text, and a
two-dimensional Scroller. Calculate and Graph share a small Rust expression
engine. This calculator language is application logic, not PAXEL; it does not
execute arbitrary Rust or JavaScript.

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
| GRAPH while already in Graph / keyboard Home while panning | Center on the origin, keeping the current zoom |
| Zoom + / − | Enlarge / shrink Calculate text; double / halve Graph magnification |
| D-pad center `edit` (when space permits) | Return to expression editing |

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

Graph starts with `sin(x)`. Edit its formula, then press Enter. Until a valid
submission, the last plotted function remains visible. Errors retain the
draft, move the cursor to the problem, and report its character column.
Drag with a mouse, scroll with a trackpad, or swipe the graph with touch.
The D-pad and arrow keys pan once the plot has focus. There is one curve.
Zoom has seven levels from 0.25× to 16×; the LCD shows the current level.
Zooming and recentering retain the formula draft without submitting it.
The `x²` and `1/x` keys insert `^2` and `^(-1)` after the current operand,
or apply to `ans` immediately after a successful calculation.
Keys always retain their opaque face and press feedback. `x` does nothing in
Calculate, and zoom keys do nothing at their respective limits.

## Expression language

| Form | Meaning / example |
| --- | --- |
| Numbers | `12`, `.5`, `2.`, `1e-3` |
| Arithmetic | `+`, `-` / `−`, `*` / `×`, `/` / `÷`, `^` |
| Grouping and sign | `(2+3)*4`, `-2^2` → `-4`, `2^-3` → `0.125` |
| Powers | Right associative: `2^3^2` → `512` |
| Constants | `pi` / `π`, `e`, and the latest successful Calculate `ans` |
| Functions | `sin(...)`, `cos(...)`, `tan(...)`, `sec(...)`, `csc(...)`, `atan(...)`, `sqrt(...)` / `√(...)`, `ln(...)`, `log(...)` / `log10(...)` |
| Variable | `x`, in Graph only; e.g. `sin(x)*x` |

All trigonometry uses **radians**. The previous basic calculator used degrees;
convert degree inputs explicitly, for example `sin(30*pi/180)`. Function names
are case insensitive. Parentheses and explicit multiplication are required:
write `2*pi` and `2*sin(x)`. `sec` and `csc` are reciprocal cosine and sine;
`atan` is inverse tangent and returns radians. `ln` is natural log; `log` is
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
keys remain at least 44 × 44 pixels. The upper bank aligns with the first
three keypad columns; the D-pad centers over the remaining two. Narrow layouts
omit its center Edit key to preserve arrow hit targets; tapping the expression
still places its cursor. Short windows scroll the whole calculator.

Native iOS and iPadOS use a viewport-width silver surface with no artificial
outer rim, shadow, or rounded corners; the device supplies the outer outline.
The web and macOS presentation retains its freestanding case. Four explicit
`DynamicIslandSpacer` components bind the live top, right, bottom, and left
insets. The content viewport reserves those edges for the status bar, island,
and home indicator, including after rotation. The silver background remains
fixed and fills the whole window. On a tall native viewport the LCD grows to
use the available height; short viewports scroll while preserving key sizes.
The spacers contribute zero inset on web and macOS.

The graph starts at 16 logical pixels per unit on both axes. Zoom ranges
from 4 to 256 pixels per unit. On resize it keeps the chosen scale, redraws
for the larger viewport, preserves its center where the finite boundary
permits, and shows more of the function. The native Scroller covers world
coordinates −256 through 256 on each axis; its pixel extent grows with zoom
(from 2,048 to 131,072 pixels square). Geometry is sampled only around the
visible viewport, with a small overscan margin. Scroll offsets are the source
of truth for both axes.

Input is limited to 256 characters and parser nesting to 32. Each plot refresh
is bounded to 4,096 output points and 250,000 expression-node visits. Interval
checks and adaptive refinement leave gaps across poles and undefined domains.
`DETAIL LIMIT` means some finite detail could not be resolved within these
bounds; rapid oscillation and ill-conditioned expressions can be incomplete.
The plot is not a proof that every root or singularity has been found.
`NO RESOLVED CURVE` can mean an empty real domain, a curve outside the viewport,
or an expression whose bounds could not establish continuous segments.
`WORLD EDGE` identifies the finite scrolling boundary.

This is a calculator-owned editor: native selection, paste, IME, and automatic
screen-reader button semantics are not implemented. Web hardware-key input
requires page/body focus. Text uses local Courier-family fonts; rasterization
can vary between platforms. WGPU provides a broad pointer-following highlight on the chassis; Piet keeps
the authored colors and bevels without lighting. The LCD stays unlit.

Debug and baked release builds were visually checked on the iPhone 16 Pro Max
simulator (iOS 18.2), in portrait and landscape: the heading stays below the
island in portrait, the LCD clears it in landscape, and the silver background
remains edge to edge. Runtime tests cover safe-area changes, parent resizing,
content-sized Stacker layout, and zero spacing on unsupported platforms.
Family-only font decoding and regular-face matching were repaired in the
Apple chassis; the LCD uses Courier New.

The iOS Scroller's duplicate tap recognizer was removed, leaving one
activation path through its touch observer and excluding nested scrollers
and native controls from ancestor forwarding. Simulator input automation did
not activate the calculator keys, so the tap fix, calculation, graph
interaction, and history scrolling still need a manual iOS interaction pass.
A release build was subsequently installed and launched on Argus, an iPhone
17 Pro Max; no physical iPad was tested.

The graph's initial blank viewport was traced to stale Scroller presentation
offsets: the app requested the origin, but the native host and tile planner
received zero. The shared Scroller now advances presentation with programmatic
scroll changes. A temporary Graph-first debug launch verified that `sin(x)`
draws before any gesture and stays centered after rotation; the normal
Calculate startup mode was restored afterward. Regression tests cover initial
positioning, recentering after native scrolling, zoom/content-extent updates,
and host-relative presentation offsets. An earlier macOS build also showed
an initially blank window until resize; that separate observation has not
been retested. Do not treat the native example as fully verified yet.

Argus then exposed a crash when zooming out from the default graph. The native
surface planner allowed a 4,096-point world to become one surface by assuming
downsampling, while the Apple host allocated it at the phone's 3x scale:
12,288 pixels square. Native plans now require the actual screen density,
retaining tiles until the content fits within the backing limits. Tests cover
zoom transitions and viewport coverage at 1x, 2x, and 3x. A temporary scripted
iOS release build completed zoom-out to 0.25x and zoom-in back through 2x,
with the graph still rendering afterward. The test driver was removed.
The simulator did not reproduce the physical crash; the corrected build
still needs verification on Argus, which was disconnected during this fix.

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
