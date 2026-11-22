# Glass bricks

Run `pax-cli run -p examples/fixtures/glass-bricks -t web` from the monorepo root.

Three independently paced cyan, magenta and amber point lights shine through an opaque-backed wall of pressed glass. At the default speed (1×), each travels right to left at 28–67 logical pixels per second. Once its full attenuation radius clears the left edge, it returns beyond the right edge with a fresh random height and speed. The glass ripples stay fixed within each brick. Resizing preserves the square pitch and the lights' relative travel.

**Brick size** scales the width and height of every square brick together, from 50% to 250% of the wall's responsive default. **Rear speed** scales each light's current and future passes from 0× to 10×. At 0× the lights hold their positions, and raising the speed resumes the same passes.

Switch to **Light mode** beside the title to see the same lights and glass over white. On white the unlit faces have a pale glass tint, rear lights color the transmitted backing, and the front pointer adds a white sheen. **Inner bezel** tunes the concentrated highlight on the recessed lip. **Outer bezel** tunes the one along the outside shoulder. Both range from 0× to 3× and start at 1×.

Choose **Rear light 1**, **2**, or **3** to edit that emitter's RGB color, intensity (0–12), and radius (10–120% of the wall width). Radius edits preserve the center and set the clearance distance for the next wrap.

Move the pointer or touch the wall to catch a white reflection. The pointer starts at the website light's settings (depth +95, radius 480, intensity 0.9). Pause holds the rear lights. Switch their group or the pointer off to isolate reflection and transmission, change the pointer to a finite strip, or move its depth through zero and watch the same light pass behind the wall. Adjust the thickness and ripple sliders to tune the pressed face and its lip. The comparison control cycles through glass, an identity capture with the same lighting dependency, and the unfiltered paint.

`GlassBricks` uses the experimental WGPU shader API. It shades a single container plane, preserves its child's alpha, and samples the source once per effect pixel. Its apparent thickness and internal ridges are procedural. A host supplies lights in the eligible scene scope and can set `pitch`, `thickness`, `roughness`, `inner_bezel`, and `outer_bezel` on the fixture primitive. Keep native content outside its capture. CPU rendering shows the selected backing paint. Rear travel pauses offscreen and on CPU.
