//! Keep a pooled DOM canvas reserved while initialization or a GPU surface owns it.
//! A timeout cannot establish this lifetime: adapter/device creation can outlive it.
use web_sys::HtmlCanvasElement;

pub(super) struct BrowserCanvasLease {
    pub canvas: HtmlCanvasElement,
}

impl BrowserCanvasLease {
    pub fn acquire(canvas: HtmlCanvasElement) -> Self {
        let owners = owner_count(&canvas);
        let _ = canvas.set_attribute("data-pax-surface-owners", &(owners + 1).to_string());
        Self { canvas }
    }
}

fn owner_count(canvas: &HtmlCanvasElement) -> u32 {
    canvas
        .get_attribute("data-pax-surface-owners")
        .and_then(|value| value.parse().ok())
        .unwrap_or(0)
}

impl Drop for BrowserCanvasLease {
    fn drop(&mut self) {
        let remaining = owner_count(&self.canvas).saturating_sub(1);
        let _ = self
            .canvas
            .set_attribute("data-pax-surface-owners", &remaining.to_string());
        if remaining == 0 {
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
