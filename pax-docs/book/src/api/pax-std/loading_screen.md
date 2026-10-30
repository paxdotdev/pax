# loading_screen
<!-- summary: Standard loading experience, authored with the same context as custom loaders. -->
<!-- tags: api, pax-std -->

Standard loading experience, authored with the same context as custom loaders.

## Structs
### `DefaultLoadingScreen`
A still Pax logo and an unlabeled progress bar on a black background.

Requires a loading scope supplying `@loading`. The first 67% of the bar
reports decoded download progress. With an unknown download size, or during
preparation, the same fill advances monotonically toward a reserved limit;
this is visual activity, not a measured initialization percentage.
A 250 ms exit transition fills the remaining bar while fading the screen
over the prepared application. Retrying starts a new progress cycle.
Failure/cancellation offers retry only when supported.
The optional `@replay` custom event is intended for loader previews; clicking
the logo never restarts an application download on its own.

Select this screen on Web with `[package.metadata.pax] loader = true`.
Startup packaging is experimental (PAX-962). An ordinary component
invocation outside a loading scope does not create a startup coordinator.

#### Properties
##### `display_fraction`
Type: `Property`<`f64`>

Visual fill before the exit timeline; maintained by the loading screen.
This is not an overall measured loading percentage.
