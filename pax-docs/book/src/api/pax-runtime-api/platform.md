# platform
<!-- summary: Platform, viewport, and coordinate-space marker types. -->
<!-- tags: api, pax-runtime-api -->

Platform, viewport, and coordinate-space marker types.

## Structs
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
