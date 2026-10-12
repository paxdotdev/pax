# shaders
<!-- summary: Experimental shader containers for GPU-rendered subtrees. -->
<!-- tags: api, pax-std -->

Experimental shader containers for GPU-rendered subtrees.

These primitives use the public `InstanceNode::shader_effect` extension point,
just as application-defined effects do. Requires the WGPU renderer.
Enable the opt-in `shaders` Cargo feature on `pax-kit` or `pax-std`.

## Submodules
- [shaders::gaussian_blur](shaders/gaussian_blur.md)
- [shaders::impressionist](shaders/impressionist.md)
