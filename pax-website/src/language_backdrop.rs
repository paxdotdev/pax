use crate::SiteTheme;
use pax_kit::*;

#[pax]
#[file("language_backdrop.pax")]
pub struct LanguageBackdrop {
    pub diameter: Property<f64>,
}

impl LanguageBackdrop {
    pub fn mount(&mut self, ctx: &NodeContext) {
        let bounds = ctx.bounds_self.clone();
        let deps = [bounds.untyped()];
        self.diameter.replace_with(LocalProperty::computed(
            move || (bounds.get().0 * 0.72).max(320.0),
            &deps,
        ));
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn paint_is_static_clipped_and_separate_from_foreground_content() {
        let paint = include_str!("language_backdrop.pax");
        assert_eq!(paint.matches("<Impressionist").count(), 1);
        assert_eq!(paint.matches("<Ellipse").count(), 6);
        assert!(paint.contains("ImpressionistMode::Signac"));
        assert!(paint.contains("phase=0"));
        assert!(!paint.contains("@tick"));
        assert!(!paint.contains("<Text"));
        assert!(!paint.contains("slot("));
    }
}
