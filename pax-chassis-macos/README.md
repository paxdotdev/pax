# pax-chassis-macos

Handles:
    - 2D rendering on macOS via CoreGraphics
    - passing tick events (a la rAF; see NSViewRepresentable)
    - Managing native user input (e.g. mouse, keyboard, camera, microphone, also form control events like 'click' on a button)
    - Rendering native text based off of commands from engine
    - Rendering native form controls based off of commands from engine

This directory also includes:

## pax-dev-harness-macos

Simple macOS app for developing Pax projects.  
Also usable as a template for packaging full-window Pax apps for macOS

Handles:
- Mounting pax-chassis-macos + cartridge to a simple Mac app, delegating full window rendering to Pax.
- Debug mode + LLDB support for debugging Pax projects on macOS
- Production mode, suitable for packaging full-window Pax apps for end-users
- TODO: register with CLI as an available dev-harness
    