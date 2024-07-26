# pax-chassis-macos/interface

Much of the interface for macOS is described in `pax-chassis-common`, and is common across iOS and macOS.

Thus, the interface for pax-chassis-macos includes only the relative complement of (`macOS \ iOS`), namely: a macOS full-app wrapper for a Pax component.

This is useful for local development on a Mac or for publishing full-app Pax-mac projects.

If you wish to use a Pax project as a SwiftUI view inside e.g. an existing Swift project,
look to the SPM package created inside `pax-chassis-common/pax-swift-cartridge`.  That same SPM package
is consumed by pax-app-macos.