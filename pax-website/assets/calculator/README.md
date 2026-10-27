# Calculator device captures

The rear devices and compact phone preview use captures of the canonical
`examples/calculator` component. The foreground phone on wide layouts is live;
the images do not create additional calculator trees. All three captures show
the initial polar graph, not a separate mockup or duplicated implementation.

Use `NativeImage` to preserve the captures' color and aspect ratio, as in the
Paxflix showcase. The device frames belong to the website, not the calculator.

These are web captures of the website's full-screen calculator route, with its
navigation header excluded. They illustrate responsive layouts, not independent
evidence of native target rendering. `macos.jpg` uses a narrow 560px content width
so the calculator fills its window instead of showing the wide-layout background
gutters. The macOS-style title bar and traffic-light controls are rendered in Pax
by `DeviceFrame kind="window"`, not baked into the capture.

## Refresh

From the monorepo root, initialize the workstation's Node toolchain and run the
website:

```sh
source ~/.zshrc
nvm use
target/debug/pax-cli run --path pax-website --target web --libdev
```

Open `/calculator/` with a 560×800 viewport for the floating window, 390×896 for
the phone, or 834×1262 for the portrait tablet. After the initial graph is visible,
set the floating-window graph to 1× with one press of its zoom `+`, then capture
the full viewport and crop off the 52px navigation header (560×748, 390×844, and
834×1210 respectively). Cropping the saved image avoids changing the live
viewport while capturing. Inspect captures before replacing `macos.jpg`, `phone.jpg`,
and `tablet.jpg`. The window's 560×792 design size includes the separate 44px
title bar; it does not stretch the calculator capture.
