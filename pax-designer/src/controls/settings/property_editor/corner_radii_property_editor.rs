use pax_engine::api::*;
use pax_engine::*;
use pax_manifest::*;
use pax_std::*;

use crate::controls::settings::AREAS_PROP;

use super::PropertyEditorData;

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
            AREAS_PROP.with(|areas| {
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
                let value = pax_engine::pax_lang::from_pax(&data.get().get_value_as_str(&ctx));
                if let Ok(value) = value {
                    let rad: RectangleCornerRadii =
                        RectangleCornerRadii::try_coerce(value).unwrap_or_default();
                    return rad;
                }
                RectangleCornerRadii::default()
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
        let Ok(r0): Result<f64, _> = event.text.parse() else {
            log::warn!("can't set corner radii to non-float");
            return;
        };
        let r1 = corner_radii.top_right.get().to_float();
        let r2 = corner_radii.bottom_left.get().to_float();
        let r3 = corner_radii.bottom_right.get().to_float();
        let cr_str = format!(
            "{{ top_left: {} top_right: {} bottom_left: {} bottom_right: {}}}",
            r0, r1, r2, r3,
        );
        if let Err(e) = self.data.get().set_value(ctx, &cr_str) {
            log::warn!("failed to write corner radii: {e}");
        }
    }

    pub fn change_1(&mut self, ctx: &NodeContext, event: Event<TextboxChange>) {
        let corner_radii = self.corner_radii.get();
        let r0 = corner_radii.top_left.get().to_float();
        let Ok(r1): Result<f64, _> = event.text.parse() else {
            log::warn!("can't set corner radii to non-float");
            return;
        };
        let r2 = corner_radii.bottom_left.get().to_float();
        let r3 = corner_radii.bottom_right.get().to_float();
        let cr_str = format!(
            "{{ top_left: {} top_right: {} bottom_left: {} bottom_right: {}}}",
            r0, r1, r2, r3,
        );
        if let Err(e) = self.data.get().set_value(ctx, &cr_str) {
            log::warn!("failed to write corner radii: {e}");
        }
    }
    pub fn change_2(&mut self, ctx: &NodeContext, event: Event<TextboxChange>) {
        let corner_radii = self.corner_radii.get();
        let r0 = corner_radii.top_left.get().to_float();
        let r1 = corner_radii.top_right.get().to_float();
        let Ok(r2): Result<f64, _> = event.text.parse() else {
            log::warn!("can't set corner radii to non-float");
            return;
        };
        let r3 = corner_radii.bottom_right.get().to_float();
        let cr_str = format!(
            "{{ top_left: {} top_right: {} bottom_left: {} bottom_right: {}}}",
            r0, r1, r2, r3,
        );
        if let Err(e) = self.data.get().set_value(ctx, &cr_str) {
            log::warn!("failed to write corner radii: {e}");
        }
    }
    pub fn change_3(&mut self, ctx: &NodeContext, event: Event<TextboxChange>) {
        let corner_radii = self.corner_radii.get();
        let r0 = corner_radii.top_left.get().to_float();
        let r1 = corner_radii.top_right.get().to_float();
        let r2 = corner_radii.bottom_left.get().to_float();
        let Ok(r3): Result<f64, _> = event.text.parse() else {
            log::warn!("can't set corner radii to non-float");
            return;
        };
        let cr_str = format!(
            "{{ top_left: {} top_right: {} bottom_left: {} bottom_right: {}}}",
            r0, r1, r2, r3,
        );
        if let Err(e) = self.data.get().set_value(ctx, &cr_str) {
            log::warn!("failed to write corner radii: {e}");
        }
    }
}
