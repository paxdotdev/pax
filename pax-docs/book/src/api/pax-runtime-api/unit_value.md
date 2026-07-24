# unit_value
<!-- summary: Normalized unit-domain values. -->
<!-- tags: api, pax-runtime-api -->

Normalized unit-domain values.

## Enums
### `UnitValue`
A scalar value in a normalized unit domain.

`UnitValue` accepts both unitless numbers and percents. It is useful for
properties whose authoring domain is "part of a whole", such as path drawing
progress where `0.5` and `50%` describe the same position.

#### Variants
##### `Unitless`([`Numeric`](/api/pax-runtime-api/pax_value/numeric.md#numeric))
Unitless normalized value, where `0.5` means halfway through the domain.

##### `Percent`([`Numeric`](/api/pax-runtime-api/pax_value/numeric.md#numeric))
Percent value, where `50%` means halfway through the domain.

#### Implementations
##### `to_clamped_unit_float`
<pre><code class="api-signature language-rust ignore">pub fn to_clamped_unit_float(&amp;self) -&gt; f64</code></pre>

Returns `to_unit_float()` clamped into the closed unit interval.

##### `to_unit_float`
<pre><code class="api-signature language-rust ignore">pub fn to_unit_float(&amp;self) -&gt; f64</code></pre>

Returns the wrapped value as a normalized unitless float.

`UnitValue::Unitless(0.5)` returns `0.5`, and
`UnitValue::Percent(50)` returns `0.5`.
