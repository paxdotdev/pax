# Native desktop acceptance

An ordinary Pax application with a user-owned Rust `main`, a root initializer,
startup data, native text editing and selection, button/checkbox state, nested
scrolling, vector animation, and reactive text updated by a Rust event handler.

On a configured Windows or Linux desktop host, use the repository CLI:

```sh
pax-cli run --path examples/fixtures/native-desktop-smoke --target windows
pax-cli run --path examples/fixtures/native-desktop-smoke --target linux
```

Use `build --release` with the same path and target to exercise the baked
cartridge. Launch the packaged executable from `.pax/build/release/<target>/`.
The package contains the launcher, a separate cartridge, native host resources,
and `pax-app.json`. Debug CLI sessions can reload `.pax` edits; Rust edits require
rebuilding and restarting. Direct Cargo runs do not start a reload server.
`PAX_HOST_SDK_DIR` can select an already-built native SDK for both CLI and direct
Cargo builds; it must contain `PaxHost.dll` on Windows or `libpax_host.so` on Linux.
The CLI copies that entire SDK directory into its package. Select a matching
Debug or Release SDK for the intended build.

The acceptance checks are visible: startup text must read “Initialized by
user-owned main”; Increment updates the count and animates the rectangle and
clipped native editor frame. Opaque and translucent vectors cross the editor's
right edge to exercise generated native masks while leaving its left side clear;
editing the textbox updates “Rust received”; checkbox state echoes in text;
both scrollers reveal their destination controls. Native selection, IME,
accessibility, resize, repeated close/reopen, debug reload, and release startup
must also be checked on each platform. This fixture does not itself establish
that those checks have passed.

The top-right PNG checks decoded image upload and source alpha. The “Bundled
Manrope” line uses `assets/fonts/Manrope.ttf`, copied from the website's existing
font asset with its SIL Open Font License. Its SHA-256 is
`3ae11c49db0455a3cc33e37d380f20fdb8c7f8b41dc07625c177e3d87a9d6ae6`;
provenance is recorded in [the website font notes](../../../pax-website/assets/fonts/README.md).
The PNG is an original 32×32 blue/gold checker fixture. Other text explicitly
requests the Arial system family; the fixture does not depend on remote font CSS.

Direct Windows Cargo launches embed TTF/OTF/TTC files from ordered asset roots
and extract them into a verified `pax-fonts/<content-hash>/` cache beside the
executable, before loading WinUI. This requires a writable executable directory
and hard-link support. Files are bounded to 16 MiB each, 64 MiB total, and 256
fonts; existing mismatched cache files produce an error rather than being
overwritten. Packaged applications already contain their resources and perform
no font extraction at startup. No build tools run during application startup.

Windows CLI packages use the Windows SDK's `mt.exe` to merge Windows 10/11
compatibility and PerMonitorV2 DPI declarations into the packaged launcher,
preserving other app-authored manifest entries. Direct Cargo launchers must
embed an application manifest themselves; the host's pre-UI DPI setup does not
replace OS compatibility declarations. The compiler's
[`windows-launcher.manifest`](../../../pax-compiler/files/windows-launcher.manifest)
contains the required declarations. Merge these into an existing application
manifest rather than replacing its other settings.
The CLI validates the complete merged manifest, preserving an authored
assembly identity. If it is absent, the CLI supplies a stable identity derived
from the launcher name and the Cargo package version. The compatibility
fragment alone is not a complete application manifest.
This fixture supplies its own [`app.manifest`](app.manifest) and a
[`build.rs`](build.rs) that embeds it only in the Windows executable. Direct
`cargo build --lib --bin native-desktop-smoke` therefore produces the required
manifest without a post-build edit. CLI packaging must preserve its `asInvoker`
execution-level declaration while merging the compatibility and DPI entries.

Run the executable with `--prepare-drop` to initialize and retire two sessions
without opening a window. This checks that prelaunch shutdown clears the active
session and permits reinitialization. It does not exercise asynchronous shutdown
resources or GPU work.

On Windows/Linux, `--prepare-drop-async` registers a diagnostic cleanup owner
whose worker acknowledges shutdown after 250 ms, then performs two prepare/drop
cycles. Each drop must wait at least 250 ms; the output must contain two cleanup
begin/acknowledgement pairs and two retired-cycle markers. This exercises the
bounded prelaunch cleanup pump without creating a window or GPU. Normal window
close remains asynchronous. Record platform results separately; this diagnostic
does not establish rendering, IME, or device-loss behavior.
