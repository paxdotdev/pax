# Pax 

Pax is a language for creating high-performance UIs that render natively on any device.

Pax is designed to support any platform via swappable rendering backends, either 2D or 3D, across Web, iOS, Android, Desktop (macOS, Linux, Windows), and embedded devices.  See the current support matrix below.

Finally, Pax is designed to be _designed._ That is:
 1. Pax's declaration format is meant to be read & written by tooling as well as by hand.
 2. Pax is designed to be _expressive,_ including support for vector drawings and complex animations, as well as plain ol' forms and layouts.  Feel free to mix and match all of the above.
 3. Pax's rendering engine is intended to offer a rendering platform for any number of future visual UI design tools — e.g. a next generation of "Figma" or "Illustrator" that paints UIs instead of pictures.

Pax is free, under a permissive license (MIT/Apache 2.0, your choice).   

## How it works

Pax attaches to a _host codebase_ which is responsible for
any imperative logic and side-effectful logic (e.g. network requests, operating system interactions.)  This divide allows Pax itself to remain highly declarative.

Currently Pax supports Rust as a host language, though support for JavaScript/TypeScript is on the [roadmap](TODO.md).

Following is a simple example.  This Pax UI describes a rectangle at the center of the viewport that can be clicked.  Upon click, the rectangle increments its rotation by 1/20 radians.

```rust
use pax::*;
use pax::drawing2D::Rectangle;

#[pax(
    @template {
        <Rectangle on_click=@self.handle_click transform=@{
            align(50%, 50%) *
            rotate(self.num_clicks / 20.0)
        }>
    }
)]
pub struct HelloWorld {
    num_clicks: isize,
}

impl HelloWorld {
    pub fn handle_click(&mut self, args: ArgsClick) {
        let old_num_clicks = self.num_clicks.get();
        self.num_clicks.set(old_num_clicks + 1);
    }
}
```

You'll notice a few moving pieces here:

#### Template and settings
  - Each component declares a template in an XML-like syntax, which describes how its UI should be displayed.
  - Any element in that template can have settings assigned as XML key-value pairs.

#### Expressions
  - Notice the two `@`-signs in the template above.  Those signal to the Pax compiler that the subsequent symbol(s) are dynamic, and should be evaluated in the context of the host codebase.  `@self.handle_click` points to a function as an event handler, and `transform=@{ ... }` calculates the contents of the `{}` block and passes a return value. 
  - The mechanism behind this is in fact a whole computer language, a sub-grammar of Pax called 'Pax Expression Language' or PAXEL for short.
  - PAXEL expressions are distinctive in a few ways:
     - Any PAXEL expression must be a pure function of its inputs and must be side-effect free
     - As a result of the above, PAXEL expressions may be aggressively cached and recalculated only when inputs change.
     - In spirit, expressions act a lot like spreadsheet formulas
     
####Components all the way down
  - This example declares a Pax component called `HelloWorld`.  Every Pax UI is a component at its root, which comprises other components in its template.  Another program or file could import `HelloWorld` and include it in its template as `<HelloWorld num_clicks=4 />`
  - Special primitives are included with Pax core and may be authored by anyone.  These primitives (`Rectangle` in the example above) have access to the core engine and drawing APIs, which is how `Rectangle` draws itself.  Other built-in primitives include `Text`, `Frame` (clipping), `Group`, `Ellipse`, and `Path`.

    
## Current status & support

Pax is in its early days but has ambitions to mature robustly.

|                             | Web browsers    | Native iOS           | Native Android | Native macOS         | Native Windows   |
|-----------------------------|-----------------|----------------------|----------------|----------------------|------------------|
| 2D rendering[1]             | ✅ <br/>(Canvas) | ✅ <br/>(CoreGraphics) | ✅ <br/>(Cairo)  | ✅ <br/>(CoreGraphics) | ✅ <br/>(Direct2D) |
| 3D rendering                | ⏲               | ⏲                    | ⏲              | ⏲                    | ⏲                |
| Vector graphics APIs        | ✅               | ⏲                    | ⏲              | ⏲                    | ⏲                |
| Native text rendering       | ✅ (via DOM)     | ⏲                    | ⏲              | ⏲                    | ⏲                |
| Native form elements        | ⏲ (via DOM)     | ⏲                    | ⏲              | ⏲                    | ⏲                |
| 2D layouts                  | ✅               | ⏲                    | ⏲              | ⏲                    | ⏲                |
| Animation APIs              | ✅               | ⏲                    | ⏲              | ⏲                    | ⏲                |
| Rust host language          | ✅ (via WASM)    | ✅                    | ✅              | ✅                    | ✅                |
| JS/TypeScript host language | ⏲               | ⏲                    | ⏲              | ⏲                    | ⏲                |

✅ Supported
⏲ Not yet supported

[1] Native 2D drawing that _just works_ on every device is available thanks to the hard work behind [piet](https://github.com/linebender/piet), 

## Anatomy of Pax


### Anatomy of a Pax UI program

1. Host Codebase
   1. Pax works as a _companion language_ to a host codebase.  As of this writing, Pax supports Rust for host codebases — i.e. you author UIs in Pax driven by the logic written in Rust. JavaScript/TypeScript host codebase support is planned for the future.
2. Pax declarations (either a "code-behind" file, or inline)
3. Pax compiler builds "cartridge", which fits (NES) into native platform renderersloaded into native codebases with SDK (a la React)








## Native rendering, native controls

Rather than introduce virtual controls at the canvas layer, Pax orchestrates a layer of native
controls as part of its rendering process.  This native overlay is used both for form controls like checkboxes
and drop-downs, as well as for rendering text.

In the browser, for example, a pool of DOM nodes is created for form control elements and text.
Those elements are positioned as an overlay on top of any canvas rendering, allowing for a cohesive
experience that blends dynamic graphics (e.g. vectors, animations) with native familiar UI elements (e.g. text boxes.)

[Visual of DOM "marionette" overlay layer on top of parallaxed graphics layer]






## Who?

Pax is managed by the Pax Foundation, a non-profit entity with open membership.  You can join and have a hand the future of Pax!


## Inspiration

Pax draws design inspiration from, among others:
 - Verilog, VHDL
 - Macromedia Flash, Dreamweaver
 - The World Wide Web, HTML, CSS
 - React, Vue
 - Visual Basic, ASP.NET
 - VisiCalc, Lotus 1-2-3, Excel
 - The Nintendo Entertainment System
 




### Running the project
`./serve.sh`

### Dev Env Setup
- Install `wasm-opt` via `binaryen`:
   ```shell
   brew install binaryen
   ```

- Install 'wasm-pack' via:
   ```shell
    curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh 
   ```
  
- Install `node`: https://nodejs.org/en/download/

- Install `yarn`:
   ```shell
  # if necessary: sudo chown -R yourusername /usr/local/lib/node_modules 
  npm i --global yarn
   ```






```
Scratch pad:

The core design goal of Pax is to be _designable._

That is, anything you could imagine _designing_ visually on a screen, e.g. through a vector
graphics design tool, or a 3D scene for a video game, should be elegantly expressible with Pax.

Further, Pax's language syntax was designed for tooling to read and write.  The compiler API accepts
parsed language tokens through the same bridge that is exposed for future design tools to perform operations like
"create vector ellipse" or "change bezier path control point."



Not
only can you express sky's-the-limit UIs with it (not unlike a game engine,)
but the language itself was designed to be "hyper-declarative"




Pax draws inspiration from hardware description languages like Verilog and VHDL, which operate
highly declaratively.

In much the same way as Verilog and VHDL allow you to "declare transitor arrangements," Pax allows
you to "declare UI arrangements," with the express goal of supporting any UI you can imagine or design.

Similar to a Hardware Description Language,[1] Pax aims to be hyper-


[1] "HDLs", like Verilog or VHDL

a tool for authoring cross-platform
native, high-performance, delightful UIs




As of the authoring of this document, Pax only supports vector and text graphics for 2D back-ends[1], but there's a clear path
to 3D support.  This would be an excellent area for community contribution; the hardest part will simply be
the creation of 3D primitives

[1] Thanks to the hard work of the folks behind `piet`




Currently Rust is the only supported host language.  Pax is itself built with Rust.  Support for JavaScript/TypeScript as a host language is on the [roadmap](TODO.md).

That is, you _describe_ the definition and properties of a UI with the Pax language, and
handle 

Currently, Pax supports


- Component definitions
- Drawing and rendering primitives, e.g. shapes, text, and form controls
- Template declarations, as compilations of components and primitives


Pax currently works alongside Rust and is built with Rust.though support for authoring UI logic in JavaScript/TypeScript is also on the 



## A bit more detail

Portable, All-purpose, eXpressive
User Interface Description Language





## How does it work?

Pax UIs are defined as `.pax` files alongside 'host language' files, either Rust or JavaScript/TypeScript.

Pax UIs and basic behaviors are expressed in the declarative Pax format, while complex, host-specific, and turing-complete logic is handled by the host language.

//TODO: explainer image showing pax & host files alongside each other
_Read more about the individual pieces of Pax in [Pax at a glance]_

When compiled, Pax files become low-level assembly code -- WASM or LLVM depending on the target platform.

This "cartridge" of binary code is then loaded into a platform-native chassis, which is loaded by a host application, e.g. by a Web app or an iOS app. 

//TODO: explainer image of NES cartridge & console





## Pax at a glance

Properties & settings (Pax)
Expressions (Pax)
Event handlers (host language)


Pax GUIs are intended to run anywhere, as stand-alone applications or embedded as components inside other applications.


This project is the first implementation of the Pax User Interface Language.  This includes:
  - Language parser, compiler, and debugging tools
  - Rendering engines for macOS, iOS, Android, Windows, and Web

## Pax's goals

Portable
Performant
Professional (license, stability)

All-purpose
Approachable
Accessible (as in accessibility)

eXtensible
eXpressive
eXhaustive (redundant with all-purpose?)

**P**ortable
- run everywhere
- tiny footprint: suitable for web applications and embedded applications
  [react, angular, Qt, etc. footprint vs. pax]
- excellent performance for a wide range of hardware
- Fast & tiny binary "cartridges" describe production applications

**A**ll-purpose
- 2D, 3D, digital documents, web apps, VR/AR, embedded GUIs
- Per-platform native UI controls (dropdowns, scroll, etc.)
- Expressive & intuitive layouts
- Animations (bouncy UI ex.), simulations (particles ex.), complex GUIs (spreadsheet ex.), data-viz (d3-style animated chart ex.)

e**X**tensible
- Reusable UI building blocks
- Open source (MIT / Apache 2.0)






Pax must latch onto another language in order to be turing complete
First, Pax is not Turing complete.  Pax relies on a companion language for Turing completeness, currently:
  - Rust,
  - TypeScript, or
  - JavaScript
Bindings could (and hopefully one day will!) be written for all programming languages.
    
[Visual: bar-chart-esque; three languages, "helmet" or layers, etc. of Pax for the UI]

Second, Pax aims to support the expression of any conceivable UI — across 2D, 3D, forms, documents, VR/AR

Second, Pax is designed to interface with design tools





<br>
*Portable, All-purpose, eXtensible*


What can be built with Pax?



  

## Some nerdier goals

  - Deterministic state system, enabling reproducible UIs (e.g. for screenshot/regression testing) 
  
  - Designable
     
  - Hackable
  


```
