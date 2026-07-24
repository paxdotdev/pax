# platform
<!-- summary: Platform, viewport, and coordinate-space marker types. -->
<!-- tags: api, pax-runtime-api -->

Platform, viewport, and coordinate-space marker types.

## Structs
### `Accel`
Current device acceleration, in meters per second squared.

On web targets, this uses `DeviceMotionEvent.accelerationIncludingGravity`
when available, falling back to `DeviceMotionEvent.acceleration`.

#### Properties
##### `x`
Type: `f64`

Acceleration along the x axis.

##### `y`
Type: `f64`

Acceleration along the y axis.

##### `z`
Type: `f64`

Acceleration along the z axis.

---

### `Gyro`
Current device orientation reported by a gyroscope/orientation sensor.

On web targets, this is sourced from `DeviceOrientationEvent` and mapped as:
`x = beta`, `y = gamma`, and `z = alpha`, all in degrees.

#### Properties
##### `x`
Type: `f64`

Front-to-back tilt, in degrees.

##### `y`
Type: `f64`

Left-to-right tilt, in degrees.

##### `z`
Type: `f64`

Compass/z-axis rotation, in degrees.

---

### `NativeLiquidGlassScope`
Runtime-inherited Apple liquid-glass effect scope.

#### Properties
##### `group_id`
Type: `u32`

Stable node id for the nearest liquid-glass scope.

##### `spacing`
Type: `f64`

Desired spacing between grouped glass surfaces, in pixels.

##### `interactive`
Type: `bool`

Whether supported Apple surfaces should use the interactive glass effect.

##### `tint`
Type: `Option`<[`Color`](/api/pax-runtime-api/color.md#color)>

Optional tint for supported Apple surfaces.

##### `variant`
Type: `String`

Apple glass style name, currently "regular" or "clear".

#### Implementations
##### `to_message`
<pre><code class="api-signature language-rust ignore">pub fn to_message(&amp;self) -&gt; <a href="/api/internal/pax-message/index.md#appleliquidglasspatch">AppleLiquidGlassPatch</a></code></pre>

Converts runtime style data into the serialized native-message payload.

---

### `TargetInfo`
Derived target facts exposed to PAXEL and Rust event handlers.

#### Properties
##### `web`
Type: `bool`

Browser-hosted rendering target.

##### `native`
Type: `bool`

Native application rendering target.

##### `ios`
Type: `bool`

Any iOS-family target, including iPhone and iPad.

##### `iphone`
Type: `bool`

iPhone-class iOS target.

##### `ipad`
Type: `bool`

iPadOS / iPad-class iOS target.

##### `macos`
Type: `bool`

macOS target.

##### `android`
Type: `bool`

Android target.

##### `windows`
Type: `bool`

Windows target.

##### `linux`
Type: `bool`

Linux target.

##### `mobile`
Type: `bool`

Any mobile OS target.

##### `desktop`
Type: `bool`

Any desktop OS target.

#### Implementations
##### `new`
<pre><code class="api-signature language-rust ignore">pub fn new(platform: <a href="/api/pax-runtime-api/platform.md#platform">Platform</a>, os: <a href="/api/pax-runtime-api/platform.md#os">OS</a>) -&gt; Self</code></pre>

Build target facts from the chassis platform and detected OS.

---

### `Viewport`
Struct representing the outermost viewport of a rendering scene, for example a browser window
or native application window.

#### Properties
##### `width`
Type: `f64`

Viewport width in pixels.

##### `height`
Type: `f64`

Viewport height in pixels.

##### `major`
Type: `f64`

Larger viewport dimension in pixels.

##### `minor`
Type: `f64`

Smaller viewport dimension in pixels.

##### `aspect`
Type: `f64`

Width divided by height. Returns 0.0 when height is 0.

##### `landscape`
Type: `bool`

True when width is greater than height.

##### `portrait`
Type: `bool`

True when height is greater than width.

##### `square`
Type: `bool`

True when width and height are effectively equal.

#### Implementations
###### `SQUARE_EPSILON`
Equality tolerance used when classifying square viewports.

##### `new`
<pre><code class="api-signature language-rust ignore">pub fn new(width: f64, height: f64) -&gt; Self</code></pre>

Build viewport facts from width and height in logical pixels.

---

### `Window`
Phantom coordinate space representing the outer window.

## Enums
### `OS`
Describes known operating systems / targets.

#### Variants
##### `Mac`
macOS.

##### `Linux`
Linux desktop.

##### `Windows`
Windows desktop.

##### `Android`
Android.

##### `IPhone`
iOS on iPhone-class devices.

##### `IPad`
iPadOS / iOS on iPad-class devices.

##### `Unknown`
OS has not been detected or reported.

#### Implementations
##### `is_android`
<pre><code class="api-signature language-rust ignore">pub fn is_android(&amp;self) -&gt; bool</code></pre>

Returns true for Android.

##### `is_desktop`
<pre><code class="api-signature language-rust ignore">pub fn is_desktop(&amp;self) -&gt; bool</code></pre>

Helper to determine if the OS is a desktop platform.

##### `is_ios`
<pre><code class="api-signature language-rust ignore">pub fn is_ios(&amp;self) -&gt; bool</code></pre>

Returns true for either iPhone-class iOS or iPadOS.

##### `is_ipad`
<pre><code class="api-signature language-rust ignore">pub fn is_ipad(&amp;self) -&gt; bool</code></pre>

Returns true for iPadOS / iPad-class iOS.

##### `is_iphone`
<pre><code class="api-signature language-rust ignore">pub fn is_iphone(&amp;self) -&gt; bool</code></pre>

Returns true for iPhone-class iOS.

##### `is_linux`
<pre><code class="api-signature language-rust ignore">pub fn is_linux(&amp;self) -&gt; bool</code></pre>

Returns true for Linux.

##### `is_macos`
<pre><code class="api-signature language-rust ignore">pub fn is_macos(&amp;self) -&gt; bool</code></pre>

Returns true for macOS.

##### `is_mobile`
<pre><code class="api-signature language-rust ignore">pub fn is_mobile(&amp;self) -&gt; bool</code></pre>

Helper to determine if the OS is a mobile platform.

##### `is_windows`
<pre><code class="api-signature language-rust ignore">pub fn is_windows(&amp;self) -&gt; bool</code></pre>

Returns true for Windows.

---

### `Platform`
Describes categories of known platforms, for differentiating certain engine behaviors.

#### Variants
##### `Web`
Browser-hosted rendering target.

##### `Native`
Native application target.

##### `Unknown`
Platform unknown or not yet reported.

#### Implementations
##### `is_native`
<pre><code class="api-signature language-rust ignore">pub fn is_native(&amp;self) -&gt; bool</code></pre>

Returns true when hosted by a native chassis.

##### `is_web`
<pre><code class="api-signature language-rust ignore">pub fn is_web(&amp;self) -&gt; bool</code></pre>

Returns true when hosted by the web chassis.
