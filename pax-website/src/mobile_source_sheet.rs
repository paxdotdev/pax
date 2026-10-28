use pax_kit::*;

#[pax]
#[file("mobile_source_sheet.pax")]
pub struct MobileSourceSheet {
    pub mounted_count: Property<usize>,
}

impl MobileSourceSheet {
    pub fn mount(&mut self, _: &NodeContext) {
        self.mounted_count.update(|count| *count += 1);
    }

    pub fn unmount(&mut self, _: &NodeContext) {
        // Exit retention owns the lifetime: keep navigation hidden and the
        // underlying page input-blocked until the sheet is actually gone.
        self.mounted_count
            .update(|count| *count = count.saturating_sub(1));
    }
}
