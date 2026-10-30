use pax_kit::*;

pax_kit::runnable! { root: crate::Main }

#[pax]
#[file("boot_screen.pax")]
pub struct BootScreen {}

impl BootScreen {
    pub fn cancel(&mut self, ctx: &NodeContext, _event: Event<ButtonClick>) {
        if let Some(loading) = ctx.loading() {
            loading.cancel();
        }
    }
}

#[pax]
#[file("main.pax")]
pub struct Main {
    pub note: Property<String>,
}
