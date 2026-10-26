use crate::bundle::DeferredPaxflix;
use crate::*;
thread_local! { static READY: Property<bool> = Property::new(false); }
#[no_mangle]
pub extern "C" fn probe_activate() {
    READY.with(|p| p.set(true));
}
#[pax]
#[file("module_gate.pax")]
pub struct WebsitePaxflix {
    pub embedded: Property<bool>,
    pub disable_keyboard: Property<bool>,
    pub ready: Property<bool>,
}
impl WebsitePaxflix {
    pub fn mount(&mut self, _: &NodeContext) {
        let ready = READY.with(|p| p.local());
        let deps = [ready.untyped()];
        self.ready
            .replace_with(LocalProperty::computed(move || ready.get(), &deps));
    }
}
