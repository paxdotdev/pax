# Pax 

Pax is a tool for creating high-performance, expressive, and portable user interfaces.  


It's part programming language,
part markup language & data exchange format, and part runtime with several platform-specific rendering engines.

One way to think of Pax is as a v2 proposal of the Web — an open interchange format, a hackable scripting format, and a low-overhead runtime that can run anywhere (in part by being backwards compatible with modern web browsers)


This project is the first implementation of the Pax User Interface Language.  This includes:
  - Language parser, compiler, and debugging tools
  - Rendering engines for macOS, iOS, Android, Windows, and Web

## What is Pax?
Pax is a way to create user interfaces.  Some of Pax's design goals:

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


