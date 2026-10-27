//! Attachment ownership is separate from command ownership. A retired texture
//! stays valid until every encoded user has been submitted or discarded.

use std::{cell::RefCell, rc::Rc};

struct OwnedTexture(wgpu::Texture);

impl Drop for OwnedTexture {
    fn drop(&mut self) {
        // Submitted work keeps its GPU-side references. No completion fence is
        // needed, but destroying before submission would invalidate that work.
        self.0.destroy();
    }
}

#[derive(Clone, Default)]
pub(super) struct AttachmentUses(Rc<RefCell<Vec<Rc<OwnedTexture>>>>);

impl AttachmentUses {
    fn retain(&self, texture: &Rc<OwnedTexture>) {
        let mut uses = self.0.borrow_mut();
        // Typically just color + stencil, plus a few capture targets. Deduplicate
        // per batch rather than retaining a reference for every drawing operation.
        if !uses.iter().any(|used| Rc::ptr_eq(used, texture)) {
            uses.push(Rc::clone(texture));
        }
    }

    pub(super) fn take(&self, commands: Vec<wgpu::CommandBuffer>) -> CommandBatch {
        CommandBatch {
            commands,
            attachments: std::mem::take(&mut *self.0.borrow_mut()),
        }
    }

    pub(super) fn discard(&self) {
        self.0.borrow_mut().clear();
    }
}

pub(super) struct Attachment {
    texture: Rc<OwnedTexture>,
    view: wgpu::TextureView,
    uses: AttachmentUses,
}

impl Attachment {
    pub(super) fn new(
        texture: wgpu::Texture,
        view: wgpu::TextureView,
        uses: &AttachmentUses,
    ) -> Self {
        Self {
            texture: Rc::new(OwnedTexture(texture)),
            view,
            uses: uses.clone(),
        }
    }

    pub(super) fn view(&self) -> &wgpu::TextureView {
        self.uses.retain(&self.texture);
        &self.view
    }

    #[cfg(test)]
    pub(super) fn texture(&self) -> &wgpu::Texture {
        &self.texture.0
    }
}

/// Encoded GPU work together with the attachment lifetimes it requires.
///
/// Merge batches with [`Self::append`] and submit through the renderer. Dropping
/// a batch discards its commands before releasing retired attachments. Raw command
/// buffers are deliberately not exposed: a detached buffer must not outlive its
/// attachment lease.
#[derive(Default)]
pub struct CommandBatch {
    // Rust drops fields in declaration order: discard commands before leases.
    commands: Vec<wgpu::CommandBuffer>,
    attachments: Vec<Rc<OwnedTexture>>,
}

impl CommandBatch {
    /// Whether this batch contains no GPU commands.
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    /// Number of encoded command buffers (not draw calls).
    pub fn len(&self) -> usize {
        self.commands.len()
    }

    /// Move another batch's commands and leases into this batch, preserving order.
    pub fn append(&mut self, other: &mut Self) {
        self.commands.append(&mut other.commands);
        self.attachments.append(&mut other.attachments);
    }

    pub(super) fn submit(mut self, queue: &wgpu::Queue) {
        queue.submit(std::mem::take(&mut self.commands));
        // Drop only after submit returns. WebGPU can now reclaim retired storage
        // as soon as the queued uses finish, independently of JavaScript GC.
    }
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;

    fn device() -> (wgpu::Device, wgpu::Queue) {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
        pollster::block_on(adapter.request_device(&Default::default())).unwrap()
    }

    fn attachment(device: &wgpu::Device, uses: &AttachmentUses, size: u32) -> Attachment {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Attachment lifetime test"),
            size: wgpu::Extent3d {
                width: size,
                height: size,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 4,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let view = texture.create_view(&Default::default());
        Attachment::new(texture, view, uses)
    }

    fn clear(device: &wgpu::Device, view: &wgpu::TextureView) -> wgpu::CommandBuffer {
        let mut encoder = device.create_command_encoder(&Default::default());
        {
            let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
        }
        encoder.finish()
    }

    fn no_validation_error(scope: wgpu::ErrorScopeGuard) {
        assert!(pollster::block_on(scope.pop()).is_none());
    }

    #[test]
    #[ignore = "requires a macOS Metal device"]
    fn retired_attachment_survives_replacement_and_detached_submission() {
        let (device, queue) = device();
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let uses = AttachmentUses::default();
        let mut current = attachment(&device, &uses, 16);
        let old = Rc::downgrade(&current.texture);
        let old_texture = current.texture.0.clone();
        let command = clear(&device, current.view());
        current = attachment(&device, &uses, 32);
        assert!(old.upgrade().is_some());
        let batch = uses.take(vec![command]);
        // The renderer and its new attachment can disappear before a combined
        // submission. The detached batch must independently preserve the old one.
        drop(current);
        drop(uses);
        assert!(old.upgrade().is_some());
        batch.submit(&queue);
        assert!(old.upgrade().is_none());
        no_validation_error(scope);

        // A surviving raw handle must not postpone explicit GPU destruction.
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let view = old_texture.create_view(&Default::default());
        queue.submit([clear(&device, &view)]);
        assert!(pollster::block_on(scope.pop()).is_some());
    }

    #[test]
    #[ignore = "requires a macOS Metal device"]
    fn attachment_reuse_and_overlapping_batches_keep_storage_live() {
        let (device, queue) = device();
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let uses = AttachmentUses::default();
        let target = attachment(&device, &uses, 16);
        let live = Rc::downgrade(&target.texture);
        let first = uses.take(vec![clear(&device, target.view())]);
        let second = uses.take(vec![clear(&device, target.view())]);
        first.submit(&queue);
        assert!(live.upgrade().is_some());
        drop(target);
        assert!(live.upgrade().is_some());
        second.submit(&queue);
        assert!(live.upgrade().is_none());
        no_validation_error(scope);
    }

    #[test]
    #[ignore = "requires a macOS Metal device"]
    fn abandoning_batch_and_encoder_releases_retired_attachments() {
        let (device, _queue) = device();
        let uses = AttachmentUses::default();
        let target = attachment(&device, &uses, 16);
        let live = Rc::downgrade(&target.texture);
        let batch = uses.take(vec![clear(&device, target.view())]);
        drop(target);
        assert!(live.upgrade().is_some());
        drop(batch);
        assert!(live.upgrade().is_none());

        let target = attachment(&device, &uses, 32);
        let live = Rc::downgrade(&target.texture);
        let command = clear(&device, target.view());
        drop(target);
        drop(command);
        uses.discard();
        assert!(live.upgrade().is_none());
    }

    #[test]
    #[ignore = "requires a macOS Metal device"]
    fn shared_submission_preserves_every_surface_and_deduplicates_uses() {
        let (device, queue) = device();
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let mut combined = CommandBatch::default();
        let mut live = Vec::new();
        for _ in 0..3 {
            let uses = AttachmentUses::default();
            let target = attachment(&device, &uses, 16);
            live.push(Rc::downgrade(&target.texture));
            let commands = (0..5).map(|_| clear(&device, target.view())).collect();
            let mut batch = uses.take(commands);
            assert_eq!(batch.attachments.len(), 1);
            combined.append(&mut batch);
        }
        assert_eq!(combined.len(), 15);
        assert!(live.iter().all(|texture| texture.upgrade().is_some()));
        combined.submit(&queue);
        assert!(live.iter().all(|texture| texture.upgrade().is_none()));
        no_validation_error(scope);
    }
}
