# pax-swift-cartridge

Holds the Swift Package Manager package + configuration for a Pax cartridge.

This is used by both macOS and iOS builds.

 1. Configures dylib dependency (built Pax+Rust cartridge)
 2. Exposes a SwiftUI View for consumption by developers or full-app chassis 

This directory is used as a codegen template — the final built and dylib-patched version of this Swift package will 
sit inside userland `.pax/pkg/pax-chassis-common/pax-swift-cartridge`