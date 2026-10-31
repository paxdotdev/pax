use pax_kit::*;
#[pax]
#[file("fixed_nav.pax")]
pub struct FixedNav {
    pub responsible: Property<bool>,
}
impl FixedNav {
    pub fn on_pre_unmount(&mut self, ctx: &NodeContext) {
        if self.responsible.get() {
            for node in ctx.visiting_nodes().all().unwrap_or_default() {
                // The original owner may also be ending during application teardown.
                let _ = node.move_to_home();
            }
        }
    }
}
