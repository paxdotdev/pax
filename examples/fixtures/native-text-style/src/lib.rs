use pax_kit::*;

pax_kit::runnable! { root: crate::NativeTextStyle }

#[pax]
#[file("lib.pax")]
pub struct NativeTextStyle {
    pub alternate: Property<bool>,
    pub single: Property<String>,
    pub multiline: Property<String>,
    pub sample_lines: Property<String>,
    pub selected: Property<u32>,
    pub radio: Property<u32>,
}

impl NativeTextStyle {
    pub fn on_mount(&mut self, _: &NodeContext) {
        self.single.set("Editable native text".into());
        self.multiline.set("Multiline native text\nSecond line".into());
        self.sample_lines.set("A longer first line\nShort line".into());
    }

    pub fn toggle_style(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.alternate.set(!self.alternate.get());
    }
}
