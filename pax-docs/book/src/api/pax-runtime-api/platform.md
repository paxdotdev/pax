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

##### `Unknown`
OS has not been detected or reported.

#### Implementations
##### `is_desktop`
<pre><code class="api-signature language-rust ignore">pub fn is_desktop(&amp;self) -&gt; bool</code></pre>

Helper to determine if the OS is a desktop platform.

##### `is_mobile`
<pre><code class="api-signature language-rust ignore">pub fn is_mobile(&amp;self) -&gt; bool</code></pre>

Helper to determine if the OS is a mobile platform.

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
