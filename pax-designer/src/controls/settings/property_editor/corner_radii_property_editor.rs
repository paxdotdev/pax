use pax_engine::api::*;
use pax_engine::*;
use pax_manifest::*;
use pax_std::*;

use super::{PropertyAreas, PropertyEditorData};

#[pax]
#[engine_import_path("pax_engine")]
#[file("controls/settings/property_editor/corner_radii_property_editor.pax")]
pub struct CornerRadiiPropertyEditor {
    pub data: Property<PropertyEditorData>,

    pub corner_radii: Property<RectangleCornerRadii>,
    pub r0: Property<String>,
    pub r1: Property<String>,
    pub r2: Property<String>,
    pub r3: Property<String>,
}

impl CornerRadiiPropertyEditor {
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        let index = self.data.get().editor_index;
        if index != 0 {
            let _ = ctx.peek_local_store(|PropertyAreas(areas): &mut PropertyAreas| {
                areas.update(|areas| {
                    while areas.len() <= index {
                        areas.push(0.0)
                    }
                    areas[index - 1] = 70.0;
                });
            });
        }
        let data = self.data.clone();
        let deps = [data.untyped()];
        let ctx = ctx.clone();
        self.corner_radii.replace_with(Property::computed(
            move || {
                data.get()
                    .get_value_typed(&ctx)
                    .map_err(|e| {
                        log::warn!(
                            "failed to read {} for {} - using default: {e}",
                            "corner radii",
                            "corner radii editor"
                        );
                    })
                    .unwrap_or_default()
            },
            &deps,
        ));

        let cr = self.corner_radii.clone();
        let deps = [cr.untyped()];
        self.r0.replace_with(Property::computed(
            move || format!("{:.1}", cr.get().top_left.get().to_float()),
            &deps,
        ));
        let cr = self.corner_radii.clone();
        self.r1.replace_with(Property::computed(
            move || format!("{:.1}", cr.get().top_right.get().to_float()),
            &deps,
        ));
        let cr = self.corner_radii.clone();
        self.r2.replace_with(Property::computed(
            move || format!("{:.1}", cr.get().bottom_left.get().to_float()),
            &deps,
        ));
        let cr = self.corner_radii.clone();
        self.r3.replace_with(Property::computed(
            move || format!("{:.1}", cr.get().bottom_right.get().to_float()),
            &deps,
        ));
    }

    pub fn change_0(&mut self, ctx: &NodeContext, event: Event<TextboxChange>) {
        let corner_radii = self.corner_radii.get();
        corner_radii.top_left.set(
            event
                .text
                .parse::<f64>()
                .unwrap_or_else(|_| {
                    log::warn!("can't set corner radii to non-float");
                    0.0
                })
                .into(),
        );
        if let Err(e) = self.data.get().set_value_typed(ctx, corner_radii) {
            log::warn!("failed to write corner radii: {e}");
        }
    }

    pub fn change_1(&mut self, ctx: &NodeContext, event: Event<TextboxChange>) {
        let corner_radii = self.corner_radii.get();
        corner_radii.top_right.set(
            event
                .text
                .parse::<f64>()
                .unwrap_or_else(|_| {
                    log::warn!("can't set corner radii to non-float");
                    0.0
                })
                .into(),
        );
        if let Err(e) = self.data.get().set_value_typed(ctx, corner_radii) {
            log::warn!("failed to write corner radii: {e}");
        }
    }
    pub fn change_2(&mut self, ctx: &NodeContext, event: Event<TextboxChange>) {
        let corner_radii = self.corner_radii.get();
        corner_radii.bottom_left.set(
            event
                .text
                .parse::<f64>()
                .unwrap_or_else(|_| {
                    log::warn!("can't set corner radii to non-float");
                    0.0
                })
                .into(),
        );
        if let Err(e) = self.data.get().set_value_typed(ctx, corner_radii) {
            log::warn!("failed to write corner radii: {e}");
        }
    }
    pub fn change_3(&mut self, ctx: &NodeContext, event: Event<TextboxChange>) {
        let corner_radii = self.corner_radii.get();
        corner_radii.bottom_right.set(
            event
                .text
                .parse::<f64>()
                .unwrap_or_else(|_| {
                    log::warn!("can't set corner radii to non-float");
                    0.0
                })
                .into(),
        );
        if let Err(e) = self.data.get().set_value_typed(ctx, corner_radii) {
            log::warn!("failed to write corner radii: {e}");
        }
    }
}
