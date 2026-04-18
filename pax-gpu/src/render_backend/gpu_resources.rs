pub(crate) fn create_multisampled_framebuffer(
    device: &wgpu::Device,
    desc: &wgpu::SurfaceConfiguration,
    sample_count: u32,
) -> (wgpu::Texture, wgpu::TextureView) {
    let multisampled_frame_descriptor = &wgpu::TextureDescriptor {
        label: Some("Multisampled frame descriptor"),
        size: wgpu::Extent3d {
            width: desc.width,
            height: desc.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count,
        dimension: wgpu::TextureDimension::D2,
        format: desc.format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[desc.format],
    };

    let texture = device.create_texture(multisampled_frame_descriptor);
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    (texture, view)
}
