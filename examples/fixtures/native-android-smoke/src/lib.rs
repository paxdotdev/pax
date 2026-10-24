use pax_kit::*;
use std::sync::atomic::{AtomicUsize, Ordering};

pax_kit::runnable! { root: crate::AndroidSmoke, init: crate::initialize }

#[pax]
#[file("lib.pax")]
pub struct AndroidSmoke {
    pub mode: Property<String>,
    pub title: Property<String>,
    pub input: Property<String>,
    pub echoed: Property<String>,
    pub clicks: Property<usize>,
    pub initialized: Property<usize>,
    pub checked: Property<bool>,
    pub scroll_y: Property<f64>,
    pub level: Property<f64>,
    pub choice: Property<u32>,
    pub radio: Property<u32>,
    pub markdown: Property<String>,
    pub code: Property<String>,
    pub image_alt: Property<bool>,
    pub images_hidden: Property<bool>,
}

pub fn initialize(context: &InitContext) -> AppResult<AndroidSmoke> {
    static INITIALIZATIONS: AtomicUsize = AtomicUsize::new(0);
    if context.startup_data() == b"fail-initializer" {
        return Err("requested initialization failure".into());
    }
    Ok(AndroidSmoke {
        mode: Property::new(std::str::from_utf8(context.startup_data())?.to_owned()),
        title: Property::new(if context.startup_data().is_empty() {
            "Pax on Android".into()
        } else {
            std::str::from_utf8(context.startup_data())?.to_owned()
        }),
        input: Property::new("Native text and IME".into()),
        echoed: Property::new("Edit the native textbox".into()),
        initialized: Property::new(INITIALIZATIONS.fetch_add(1, Ordering::SeqCst) + 1),
        markdown: Property::new("# Native Markdown\n\nA **bold** and *italic* 🦀.\n\n1. first\n2. second\n\n`a < b`".into()),
        code: Property::new(concat!("<pre data-pax-code-markup=\"example-host\" style=\"color:#202830;\">",
            "  &lt;<span style=\"color:#135ea8;\">Text</span> text=<span style=\"color:#a83e13;\">&quot;🦀 &amp;lt;&quot;</span>/&gt;\n",
            "    second line\n</pre>").into()),
        ..Default::default()
    })
}
impl AndroidSmoke {
    pub fn swap_images(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.image_alt.set(!self.image_alt.get());
    }
    pub fn toggle_images(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.images_hidden.set(!self.images_hidden.get());
    }
    pub fn toggle_emphasis(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.markdown.set(
            "# Native Markdown\n\nA *bold* and **italic** 🦀.\n\n1. first\n2. second\n\n`a < b`"
                .into(),
        );
    }
    pub fn increment(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.clicks.set(self.clicks.get() + 1);
    }
    pub fn edit(&mut self, _: &NodeContext, event: Event<TextboxInput>) {
        let text = if self.mode.get() == "uppercase" {
            let transformed = event.text.to_uppercase();
            self.input.set(transformed.clone());
            transformed
        } else {
            event.text.clone()
        };
        self.echoed.set(format!("Rust: {text}"));
    }
}
