use std::sync::atomic::{AtomicU32, Ordering};

use pax_engine::api::*;
use pax_engine::math::Generic;
use pax_engine::node_layout::LayoutProperties;
use pax_engine::*;
use pax_manifest::*;
use pax_std::*;

use crate::controls::settings::color_picker::ColorPicker;
use crate::model;
use crate::model::action::orm::{NodeLayoutSettings, SetNodeLayout};

use super::{PropertyAreas, PropertyEditorData};
mod font_options;

#[pax]
#[engine_import_path("pax_engine")]
#[file("controls/settings/property_editor/text_style_property_editor.pax")]
pub struct TextStylePropertyEditor {
    pub data: Property<PropertyEditorData>,

    // source of truth
    pub text_style: Property<TextStyle>,

    // displays
    pub font_families: Property<Vec<String>>,
    pub font_family_index: Property<u32>,
    pub font_weights: Property<Vec<String>>,
    pub font_weight_index: Property<u32>,
    pub h_align_index: Property<u32>,
    pub v_align_index: Property<u32>,

    // TODO
    pub font_size_str: Property<String>,
    pub font_color: Property<Color>,

    pub property_listeners: Property<bool>,
    pub external_change: Property<bool>,
}

impl TextStylePropertyEditor {
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        let index = self.data.get().editor_index;
        if index != 0 {
            let _ = ctx.peek_local_store(|PropertyAreas(areas): &mut PropertyAreas| {
                areas.update(|areas| {
                    while areas.len() <= index {
                        areas.push(0.0)
                    }
                    areas[index - 1] = 150.0;
                });
            });
        }
        // available options in dropdowns
        self.font_families.replace_with(Property::new(
            font_options::FONT_FAMILIES
                .iter()
                .map(|(family, _)| family.to_string())
                .collect(),
        ));

        self.font_weights.replace_with(Property::new(
            font_options::FONT_WEIGHTS
                .iter()
                .map(|(w, _)| w.to_string())
                .collect(),
        ));

        let data = self.data.clone();
        let deps = [data.untyped()];
        let data = self.data.clone();
        let ctxc = ctx.clone();
        let external_change = self.external_change.clone();
        // source of truth, everything else syncs to this
        self.text_style.replace_with(Property::computed(
            move || {
                external_change.set(true);
                data.get()
                    .get_value_typed(&ctxc)
                    .map_err(|e| {
                        log::warn!(
                            "failed to read {} for {} - using default: {e}",
                            "text style",
                            "text style editor"
                        );
                    })
                    .unwrap_or_default()
            },
            &deps,
        ));

        let text_style = self.text_style.clone();
        let deps = [text_style.untyped()];

        // sync indices (family, weight) with current text style
        let families = self.font_families.clone();
        let ts = text_style.clone();
        self.font_family_index.replace_with(Property::computed(
            move || {
                let Font::Web(family, _, _, _) = ts.get().font.get();
                let res = families
                    .read(|f| {
                        f.iter()
                            .enumerate()
                            .find_map(|(i, e)| (e == &family).then_some(i as u32))
                    })
                    .unwrap_or(3);
                res
            },
            &deps,
        ));

        let ts = text_style.clone();
        self.font_weight_index.replace_with(Property::computed(
            move || {
                let Font::Web(_, _, _, weight) = ts.get().font.get();
                font_options::FONT_WEIGHTS
                    .iter()
                    .enumerate()
                    .find_map(|(i, (_, e))| (e == &weight).then_some(i as u32))
                    .unwrap_or(3)
            },
            &deps,
        ));

        let ts = text_style.clone();
        self.font_size_str.replace_with(Property::computed(
            move || match ts.get().font_size.get() {
                Size::Pixels(px) => format!("{}px", px),
                Size::Percent(perc) => format!("{}%", perc),
                Size::Combined(px, perc) => format!("{}px + {}%", px, perc),
            },
            &deps,
        ));

        let ts = text_style.clone();
        self.font_color.replace_with(Property::computed(
            move || match ts.get().fill.get() {
                Fill::Solid(color) => color,
                _ => Default::default(),
            },
            &deps,
        ));

        let ts = text_style.clone();
        self.v_align_index.replace_with(Property::computed(
            move || match ts.get().align_vertical.get() {
                TextAlignVertical::Top => 0,
                TextAlignVertical::Center => 1,
                TextAlignVertical::Bottom => 2,
            },
            &deps,
        ));

        let ts = text_style.clone();
        self.h_align_index.replace_with(Property::computed(
            move || match ts.get().align_horizontal.get() {
                TextAlignHorizontal::Left => 0,
                TextAlignHorizontal::Center => 1,
                TextAlignHorizontal::Right => 2,
            },
            &deps,
        ));

        // save and trigger listeners if dirty on tick
        let deps = [
            self.font_family_index.untyped(),
            self.font_weight_index.untyped(),
            self.font_size_str.untyped(),
            self.v_align_index.untyped(),
            self.h_align_index.untyped(),
            self.font_color.untyped(),
        ];
        let font_family_index = self.font_family_index.clone();
        let font_weight_index = self.font_weight_index.clone();
        let font_size = self.font_size_str.clone();
        let v_align_index = self.v_align_index.clone();
        let h_align_index = self.h_align_index.clone();
        let font_color = self.font_color.clone();

        let cctx = ctx.clone();
        let data = self.data.clone();
        let external_change = self.external_change.clone();
        self.property_listeners.replace_with(Property::computed(
            move || {
                // NOTE: don't move these to inside the external change check,
                // since calling get on these props is what set's that value if text_style change
                // was from "outside"
                let (family, url) = font_options::FONT_FAMILIES[font_family_index.get() as usize];
                let (_, ref weight) = font_options::FONT_WEIGHTS[font_weight_index.get() as usize];
                let font_size_value = pax_engine::pax_lang::from_pax(&font_size.get());
                let mut font_size = Size::default();
                if !font_size_value.is_err() {
                    font_size = Size::try_coerce(font_size_value.unwrap()).unwrap_or_default();
                }

                let h_align = match h_align_index.get() {
                    0 => TextAlignHorizontal::Left,
                    1 => TextAlignHorizontal::Center,
                    2 => TextAlignHorizontal::Right,
                    _ => unreachable!("index out of bounds"),
                };
                let v_align = match v_align_index.get() {
                    0 => TextAlignVertical::Top,
                    1 => TextAlignVertical::Center,
                    2 => TextAlignVertical::Bottom,
                    _ => unreachable!("index out of bounds"),
                };
                let font_color = font_color.get();
                if !external_change.get() {
                    let text_style = TextStyle {
                        font: Property::new(Font::Web(
                            family.to_string(),
                            url.to_string(),
                            FontStyle::Normal,
                            weight.clone(),
                        )),
                        font_size: Property::new(font_size),
                        fill: Property::new(Fill::Solid(font_color)),
                        underline: Property::new(false),
                        align_multiline: Property::new(h_align.clone()),
                        align_vertical: Property::new(v_align.clone()),
                        align_horizontal: Property::new(h_align.clone()),
                    };
                    Self::update_textstyle(&data, text_style, &cctx);
                }
                external_change.set(false);
                true
            },
            &deps,
        ));
    }

    fn update_textstyle(
        data: &Property<PropertyEditorData>,
        text_style: TextStyle,
        ctx: &NodeContext,
    ) {
        if let Err(e) = data.get().set_value_typed(ctx, text_style) {
            log::warn!("failed to write textstyle: {e}");
        }
    }

    pub fn pre_render(&mut self, _ctx: &NodeContext) {
        // trigger dirty gets, if deps have changed
        self.property_listeners.get();
    }

    pub fn size_change(&mut self, _ctx: &NodeContext, event: Event<TextboxChange>) {
        self.font_size_str.set(event.text.clone());
    }

    pub fn h_align_left(&mut self, _ctx: &NodeContext, _event: Event<Click>) {
        self.h_align_index.set(0);
    }
    pub fn h_align_center(&mut self, _ctx: &NodeContext, _event: Event<Click>) {
        self.h_align_index.set(1);
    }
    pub fn h_align_right(&mut self, _ctx: &NodeContext, _event: Event<Click>) {
        self.h_align_index.set(2);
    }

    pub fn v_align_top(&mut self, _ctx: &NodeContext, _event: Event<Click>) {
        self.v_align_index.set(0);
    }
    pub fn v_align_center(&mut self, _ctx: &NodeContext, _event: Event<Click>) {
        self.v_align_index.set(1);
    }
    pub fn v_align_bottom(&mut self, _ctx: &NodeContext, _event: Event<Click>) {
        self.v_align_index.set(2);
    }
}
