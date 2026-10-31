use pax_kit::*;
use std::sync::atomic::{AtomicUsize, Ordering};
static NEXT_MOUNT: AtomicUsize = AtomicUsize::new(1);

#[pax]
#[file("identity_logo.pax")]
pub struct IdentityLogo {
    pub mount_number: Property<usize>,
    pub count: Property<usize>,
    pub note: Property<String>,
    pub logo_scale: Property<f64>,
}
impl IdentityLogo {
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        self.mount_number
            .set(NEXT_MOUNT.fetch_add(1, Ordering::Relaxed));
        self.note.set("Edit me, then move me".into());
        let bounds = ctx.bounds_self.clone();
        let deps = [bounds.untyped()];
        self.logo_scale.replace_with(LocalProperty::computed(
            move || {
                let (width, height) = bounds.get();
                ((width * 0.48) / 920.72).min((height * 0.86) / 463.95)
            },
            &deps,
        ));
    }
    pub fn increment(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.count.set(self.count.get() + 1);
    }
    pub fn edit(&mut self, _: &NodeContext, event: Event<TextboxInput>) {
        self.note.set(event.text.clone());
    }
}
