//! Keep a pooled DOM canvas reserved while initialization or a GPU surface owns it.
//! A timeout cannot establish this lifetime: adapter/device creation can outlive it.
#[cfg(target_arch = "wasm32")]
use web_sys::HtmlCanvasElement;

#[cfg(target_arch = "wasm32")]
pub(super) struct BrowserCanvasLease {
    pub canvas: HtmlCanvasElement,
    presentation: CanvasPresentation,
    private: Option<web_sys::OffscreenCanvas>,
}

#[cfg(target_arch = "wasm32")]
impl BrowserCanvasLease {
    pub fn acquire(
        canvas: HtmlCanvasElement,
        width: u32,
        height: u32,
    ) -> Result<Self, anyhow::Error> {
        let private = canvas
            .has_attribute("data-pax-bitmap-front")
            .then(|| web_sys::OffscreenCanvas::new(width, height))
            .transpose()
            .map_err(|error| anyhow::anyhow!("private WebGPU canvas: {error:?}"))?;
        let presentation =
            CanvasPresentation::new(canvas.id(), canvas.get_attribute("data-surface-generation"));
        if private.is_none() {
            let _ = canvas.style().set_property("visibility", "hidden");
        }
        let owners = owner_count(&canvas);
        let _ = canvas.set_attribute("data-pax-surface-owners", &(owners + 1).to_string());
        super::record_browser_surface_event(&canvas, "initialization-start");
        Ok(Self {
            canvas,
            presentation,
            private,
        })
    }

    pub fn target(&self) -> wgpu::SurfaceTarget<'static> {
        match &self.private {
            Some(canvas) => wgpu::SurfaceTarget::OffscreenCanvas(canvas.clone()),
            None => wgpu::SurfaceTarget::Canvas(self.canvas.clone()),
        }
    }

    pub fn resize(&self, width: u32, height: u32) {
        if let Some(canvas) = &self.private {
            // The visible backing retains its displayed dimensions until the
            // host installs this image. Report configuration of the GPU target.
            let _ = self
                .canvas
                .set_attribute("data-pax-prepared-width", &width.to_string());
            let _ = self
                .canvas
                .set_attribute("data-pax-prepared-height", &height.to_string());
            if canvas.width() != width {
                canvas.set_width(width);
            }
            if canvas.height() != height {
                canvas.set_height(height);
            }
        } else {
            super::sync_browser_canvas_backing_size(&self.canvas, width, height);
        }
    }

    pub fn present(&mut self) {
        if let Some(private) = &self.private {
            if self.canvas.id() != self.presentation.id
                || self.canvas.get_attribute("data-surface-generation")
                    != self.presentation.generation
            {
                return;
            }
            // Take ownership before returning to the browser. Reading the swap
            // chain from a completion callback could capture a subsequent draw.
            let bitmap = private
                .transfer_to_image_bitmap()
                .expect("take submitted tile");
            let options = web_sys::CustomEventInit::new();
            options.set_detail(&bitmap);
            options.set_bubbles(true);
            options.set_cancelable(true);
            let event =
                web_sys::CustomEvent::new_with_event_init_dict("pax-tile-submission", &options)
                    .expect("create tile submission");
            if self.canvas.dispatch_event(&event).unwrap_or(true) {
                // No owner claimed the bitmap (e.g. the application closed).
                bitmap.close();
            }
            return;
        }
        // No per-frame DOM reads/writes after first presentation. Queue submission
        // precedes this call on both ordinary and deferred rendering paths; no GPU
        // wait or extra clear submission is needed to replace the previous image.
        if self.presentation.pending
            && self.presentation.submit(
                &self.canvas.id(),
                self.canvas
                    .get_attribute("data-surface-generation")
                    .as_deref(),
            )
        {
            let _ = self.canvas.style().remove_property("visibility");
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn owner_count(canvas: &HtmlCanvasElement) -> u32 {
    canvas
        .get_attribute("data-pax-surface-owners")
        .and_then(|value| value.parse().ok())
        .unwrap_or(0)
}

#[cfg(target_arch = "wasm32")]
impl Drop for BrowserCanvasLease {
    fn drop(&mut self) {
        let remaining = owner_count(&self.canvas).saturating_sub(1);
        let _ = self
            .canvas
            .set_attribute("data-pax-surface-owners", &remaining.to_string());
        if remaining == 0 {
            super::record_browser_surface_event(&self.canvas, "lease-released");
            let options = web_sys::EventInit::new();
            options.set_bubbles(true);
            if let Ok(event) =
                web_sys::Event::new_with_event_init_dict("pax-surface-released", &options)
            {
                let _ = self.canvas.dispatch_event(&event);
            }
        }
    }
}

// The owner identity is captured before async initialization, not at completion.
// Retirement clears the id; reuse changes the generation, even for the same id.
struct CanvasPresentation {
    id: String,
    generation: Option<String>,
    pending: bool,
}

impl CanvasPresentation {
    fn new(id: String, generation: Option<String>) -> Self {
        Self {
            id,
            generation,
            pending: true,
        }
    }

    fn submit(&mut self, id: &str, generation: Option<&str>) -> bool {
        if !self.pending || self.id != id || self.generation.as_deref() != generation {
            return false;
        }
        self.pending = false;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::CanvasPresentation;

    #[test]
    fn reveal_once_after_current_owner_submits() {
        let mut owner = CanvasPresentation::new("tile".into(), Some("1".into()));
        assert!(
            owner.pending,
            "initialization alone must not reveal old pixels"
        );
        assert!(owner.submit("tile", Some("1")));
        assert!(
            !owner.submit("tile", Some("1")),
            "retain same-owner coverage without style churn"
        );
    }

    #[test]
    fn retired_or_reassigned_owner_cannot_reveal() {
        let mut owner = CanvasPresentation::new("tile".into(), Some("1".into()));
        assert!(
            !owner.submit("", Some("1")),
            "retirement clears the logical id"
        );
        assert!(!owner.submit("other", Some("1")));
        assert!(
            !owner.submit("tile", Some("2")),
            "same-id reuse is a new owner"
        );
        assert!(!owner.submit("tile", None));
        assert!(owner.pending);
        let mut replacement = CanvasPresentation::new("tile".into(), Some("2".into()));
        assert!(replacement.submit("tile", Some("2")));
    }

    #[test]
    fn standalone_canvas_without_pool_generation_can_present() {
        let mut owner = CanvasPresentation::new(String::new(), None);
        assert!(owner.submit("", None));
        assert!(!owner.submit("", None));
    }
}
