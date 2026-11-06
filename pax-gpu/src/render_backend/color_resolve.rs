use super::{attachments::Attachment, Cell, TextureView};

/// Validity of one resolved destination relative to its multisampled attachment.
/// This tracks encoded command order, not GPU completion. Keep it with the exact
/// attachment pair: a fresh surface image must not inherit the previous image's
/// validity, even when the multisampled backing is retained.
pub(super) struct ColorResolve {
    dirty: Cell<bool>,
    #[cfg(test)]
    pub(super) encoded_resolves: Cell<usize>,
}

impl Default for ColorResolve {
    fn default() -> Self {
        Self {
            dirty: Cell::new(true),
            #[cfg(test)]
            encoded_resolves: Cell::new(0),
        }
    }
}

impl ColorResolve {
    pub(super) fn needs_resolve(&self) -> bool {
        self.dirty.get()
    }

    pub(super) fn write_view<'a>(
        &self,
        destination: &'a TextureView,
        multisampled: Option<&'a Attachment>,
    ) -> &'a TextureView {
        if let Some(multisampled) = multisampled {
            self.dirty.set(true);
            multisampled.view()
        } else {
            destination
        }
    }

    /// Call immediately before a single-sample consumer, in the same ordered
    /// command stream as its writes. Store preserves MSAA samples for subsequent
    /// partial paint; a CPU yield or encoder boundary alone is not a consumer.
    pub(super) fn encode(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        destination: &TextureView,
        multisampled: Option<&Attachment>,
    ) {
        let Some(multisampled) = multisampled else {
            return;
        };
        if !self.dirty.get() {
            return;
        }
        {
            let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Resolve color for consumption"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: multisampled.view(),
                    depth_slice: None,
                    resolve_target: Some(destination),
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
        }
        self.dirty.set(false);
        #[cfg(test)]
        self.encoded_resolves.set(self.encoded_resolves.get() + 1);
    }
}
