use crate::{chamfer, InspectionData};
#[allow(unused_imports)]
use crate::{connection_link::ConnectionLink, doc_link::DocLink};
use pax_kit::*;

#[pax]
#[file("inspector.pax")]
pub struct Inspector {
    pub panel: Property<InspectionData>,
    pub selected: Property<String>,
    pub retired: Property<Vec<u64>>,
    pub outline: Property<Vec<PathElement>>,
}
impl Inspector {
    pub fn mount(&mut self, ctx: &NodeContext) {
        let bounds = ctx.bounds_self.clone();
        self.outline.replace_with(Property::computed(
            move || {
                let (w, h) = bounds.get();
                chamfer(w, h, 14.)
            },
            &[ctx.bounds_self.untyped()],
        ));
    }
    pub fn close(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.selected.set(String::new());
    }
    pub fn unmount(&mut self, _: &NodeContext) {
        // Retire the outer stacking layer only after Pax finishes the exit.
        let mut retired = self.retired.get();
        retired.push(self.panel.get().serial);
        self.retired.set(retired);
    }
}
