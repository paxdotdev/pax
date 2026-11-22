# forms::theme_toggle
<!-- summary: API docs for pax-std::forms::theme_toggle. -->
<!-- tags: api, pax-std -->

## Structs
### `ThemeToggle`
A moon/sun switch with a native button for pointer and keyboard activation.
Bind `light_mode` to the preference the switch should edit. The 69×23px
artwork is centered inside the bounds supplied for the hit target.

#### Properties
##### `light_mode`
Type: `Property`<`bool`>

Selected appearance: false for dark, true for light.

##### `chrome_light_mode`
Type: `Property`<`bool`>

Draw dark icons for a light surrounding surface, independently of selection.

##### `label`
Type: `Property`<`String`>

Optional accessible action label; by default it names the theme to switch to.
