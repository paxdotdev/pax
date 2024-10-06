use std::sync::atomic::{AtomicU32, Ordering};

use pax_engine::api::*;
use pax_engine::math::Generic;
use pax_engine::node_layout::LayoutProperties;
use pax_engine::*;
use pax_manifest::*;
use pax_std::*;

use crate::controls::settings::color_picker::ColorPicker;
use crate::controls::settings::AREAS_PROP;
use crate::model;
use crate::model::action::orm::{NodeLayoutSettings, SetNodeLayout};

use super::PropertyEditorData;

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
    pub auto: Property<bool>,

    // TODO
    pub font_size: Property<String>,
    pub font_color: Property<Color>,

    pub property_listeners: Property<bool>,
    pub external_change: Property<bool>,
}

const FONT_FAMILIES: &[(&str, &str)] = &[
    (
        "Alegreya",
        "https://fonts.googleapis.com/css2?family=Alegreya:ital,wght@0,400;0,500;0,600;0,700;0,800;0,900;1,400;1,500;1,600;1,700;1,800;1,900&display=swap",
    ),
    (
        "Arimo",
        "https://fonts.googleapis.com/css2?family=Arimo:ital,wght@0,400;0,500;0,600;0,700;1,400;1,500;1,600;1,700&display=swap",
    ),
    (
        "Cormorant Garamond",
        "https://fonts.googleapis.com/css2?family=Cormorant+Garamond:ital,wght@0,300;0,400;0,500;0,600;0,700;1,300;1,400;1,500;1,600;1,700&display=swap",
    ),
    (
        "Crimson Text",
        "https://fonts.googleapis.com/css2?family=Crimson+Text:ital,wght@0,400;0,600;0,700;1,400;1,600;1,700&display=swap",
    ),
    (
        "Fira Code",
        "https://fonts.googleapis.com/css2?family=Fira+Code:wght@300;400;500;600;700&display=swap",
    ),
    (
        "Fira Sans",
        "https://fonts.googleapis.com/css2?family=Fira+Sans:ital,wght@0,100;0,200;0,300;0,400;0,500;0,600;0,700;0,800;0,900;1,100;1,200;1,300;1,400;1,500;1,600;1,700;1,800;1,900&display=swap",
    ),
    (
        "IBM Plex Mono",
        "https://fonts.googleapis.com/css2?family=IBM+Plex+Mono:ital,wght@0,100;0,200;0,300;0,400;0,500;0,600;0,700;1,100;1,200;1,300;1,400;1,500;1,600;1,700&display=swap",
    ),
    (
        "IBM Plex Sans",
        "https://fonts.googleapis.com/css2?family=IBM+Plex+Sans:ital,wght@0,100;0,200;0,300;0,400;0,500;0,600;0,700;1,100;1,200;1,300;1,400;1,500;1,600;1,700&display=swap",
    ),
    (
        "IBM Plex Serif",
        "https://fonts.googleapis.com/css2?family=IBM+Plex+Serif:ital,wght@0,100;0,200;0,300;0,400;0,500;0,600;0,700;1,100;1,200;1,300;1,400;1,500;1,600;1,700&display=swap",
    ),
    (
        "Inconsolata",
        "https://fonts.googleapis.com/css2?family=Inconsolata:wght@200..900&display=swap",
    ),
    (
        "Inter",
        "https://fonts.googleapis.com/css2?family=Inter:wght@100..900&display=swap",
    ),
    (
        "JetBrains Mono",
        "https://fonts.googleapis.com/css2?family=JetBrains+Mono:ital,wght@0,100;0,200;0,300;0,400;0,500;0,600;0,700;0,800;1,100;1,200;1,300;1,400;1,500;1,600;1,700;1,800&display=swap",
    ),
    (
        "Lato",
        "https://fonts.googleapis.com/css2?family=Lato:ital,wght@0,100;0,300;0,400;0,700;0,900;1,100;1,300;1,400;1,700;1,900&display=swap",
    ),
    (
        "Lora",
        "https://fonts.googleapis.com/css2?family=Lora:ital,wght@0,400;0,500;0,600;0,700;1,400;1,500;1,600;1,700&display=swap",
    ),
    (
        "Merriweather",
        "https://fonts.googleapis.com/css2?family=Merriweather:ital,wght@0,300;0,400;0,700;0,900;1,300;1,400;1,700;1,900&display=swap",
    ),
    (
        "Montserrat",
        "https://fonts.googleapis.com/css2?family=Montserrat:ital,wght@0,100;0,200;0,300;0,400;0,500;0,600;0,700;0,800;0,900;1,100;1,200;1,300;1,400;1,500;1,600;1,700;1,800;1,900&display=swap",
    ),
    (
        "Noto Sans",
        "https://fonts.googleapis.com/css2?family=Noto+Sans:ital,wght@0,100;0,200;0,300;0,400;0,500;0,600;0,700;0,800;0,900;1,100;1,200;1,300;1,400;1,500;1,600;1,700;1,800;1,900&display=swap",
    ),
    (
        "Noto Serif",
        "https://fonts.googleapis.com/css2?family=Noto+Serif:ital,wght@0,100;0,200;0,300;0,400;0,500;0,600;0,700;0,800;0,900;1,100;1,200;1,300;1,400;1,500;1,600;1,700;1,800;1,900&display=swap",
    ),
    (
        "Open Sans",
        "https://fonts.googleapis.com/css2?family=Open+Sans:ital,wght@0,300;0,400;0,500;0,600;0,700;0,800;1,300;1,400;1,500;1,600;1,700;1,800&display=swap",
    ),
    (
        "Oswald",
        "https://fonts.googleapis.com/css2?family=Oswald:wght@200..700&display=swap",
    ),
    (
        "Playfair Display",
        "https://fonts.googleapis.com/css2?family=Playfair+Display:ital,wght@0,400;0,500;0,600;0,700;0,800;0,900;1,400;1,500;1,600;1,700;1,800;1,900&display=swap",
    ),
    (
        "PT Sans",
        "https://fonts.googleapis.com/css2?family=PT+Sans:ital,wght@0,400;0,700;1,400;1,700&display=swap",
    ),
    (
        "PT Serif",
        "https://fonts.googleapis.com/css2?family=PT+Serif:ital,wght@0,400;0,700;1,400;1,700&display=swap",
    ),
    (
        "Raleway",
        "https://fonts.googleapis.com/css2?family=Raleway:ital,wght@0,100;0,200;0,300;0,400;0,500;0,600;0,700;0,800;0,900;1,100;1,200;1,300;1,400;1,500;1,600;1,700;1,800;1,900&display=swap",
    ),
    (
        "Roboto",
        "https://fonts.googleapis.com/css2?family=Roboto:ital,wght@0,100;0,300;0,400;0,500;0,700;0,900;1,100;1,300;1,400;1,500;1,700;1,900&display=swap",
    ),
    (
        "Roboto Mono",
        "https://fonts.googleapis.com/css2?family=Roboto+Mono:ital,wght@0,100;0,200;0,300;0,400;0,500;0,600;0,700;1,100;1,200;1,300;1,400;1,500;1,600;1,700&display=swap",
    ),
    (
        "Roboto Slab",
        "https://fonts.googleapis.com/css2?family=Roboto+Slab:wght@100;200;300;400;500;600;700;800;900&display=swap",
    ),
    (
        "Source Code Pro",
        "https://fonts.googleapis.com/css2?family=Source+Code+Pro:ital,wght@0,200;0,300;0,400;0,500;0,600;0,700;0,800;0,900;1,200;1,300;1,400;1,500;1,600;1,700;1,800;1,900&display=swap",
    ),
    (
        "Source Sans Pro",
        "https://fonts.googleapis.com/css2?family=Source+Sans+Pro:ital,wght@0,200;0,300;0,400;0,600;0,700;0,900;1,200;1,300;1,400;1,600;1,700;1,900&display=swap",
    ),
    (
        "Ubuntu",
        "https://fonts.googleapis.com/css2?family=Ubuntu:ital,wght@0,300;0,400;0,500;0,700;1,300;1,400;1,500;1,700&display=swap",
    ),
    (
        "Ubuntu Mono",
        "https://fonts.googleapis.com/css2?family=Ubuntu+Mono:ital,wght@0,400;0,700;1,400;1,700&display=swap",
    ),
    (
        "Unknown",
        "",
    ),
    (
       "ff-real-headline-pro",
       "https://use.typekit.net/ivu7epf.css",        
    ),
];

const FONT_WEIGHTS: &[(&str, FontWeight)] = &[
    ("Thin", FontWeight::Thin),
    ("ExtraLight", FontWeight::ExtraLight),
    ("Light", FontWeight::Light),
    ("Normal", FontWeight::Normal),
    ("Medium", FontWeight::Medium),
    ("SemiBold", FontWeight::SemiBold),
    ("Bold", FontWeight::Bold),
    ("ExtraBold", FontWeight::ExtraBold),
    ("Black", FontWeight::Black),
];

impl TextStylePropertyEditor {
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        let index = self.data.get().editor_index;
        if index != 0 {
            AREAS_PROP.with(|areas| {
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
            FONT_FAMILIES
                .iter()
                .map(|(family, _)| family.to_string())
                .collect(),
        ));

        self.font_weights.replace_with(Property::new(
            FONT_WEIGHTS.iter().map(|(w, _)| w.to_string()).collect(),
        ));

        let data = self.data.clone();
        let deps = [data.untyped()];
        let cctx = ctx.clone();
        self.auto.replace_with(Property::computed(
            move || {
                let data = data.get();
                let uid = UniqueTemplateNodeIdentifier::build(data.stid, data.snid);
                let node = cctx.get_nodes_by_global_id(uid).into_iter().next().unwrap();
                let layout = node.layout_properties();
                layout.width.is_none() && layout.height.is_none()
            },
            &deps,
        ));
        let data = self.data.clone();
        let cctx = ctx.clone();
        let external_change = self.external_change.clone();
        // source of truth, everything else syncs to this
        self.text_style.replace_with(Property::computed(
            move || {
                external_change.set(true);
                let pax = &data.get().get_value_as_str(&cctx);
                let value = pax_engine::pax_lang::from_pax(pax);
                if let Ok(value) = value {
                    let style: Result<TextStyle, String> = TextStyle::try_coerce(value);
                    return style.unwrap_or_default();
                }
                TextStyle::default()
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
                FONT_WEIGHTS
                    .iter()
                    .enumerate()
                    .find_map(|(i, (_, e))| (e == &weight).then_some(i as u32))
                    .unwrap_or(3)
            },
            &deps,
        ));

        let ts = text_style.clone();
        self.font_size.replace_with(Property::computed(
            move || match ts.get().font_size.get() {
                Size::Pixels(px) => format!("{}px", px),
                Size::Percent(perc) => format!("{}%", perc),
                Size::Combined(px, perc) => format!("{}px + {}%", px, perc),
            },
            &deps,
        ));

        let ts = text_style.clone();
        self.font_color
            .replace_with(Property::computed(move || ts.get().fill.get(), &deps));

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
            self.font_size.untyped(),
            self.v_align_index.untyped(),
            self.h_align_index.untyped(),
            self.font_color.untyped(),
        ];
        let font_family_index = self.font_family_index.clone();
        let font_weight_index = self.font_weight_index.clone();
        let font_size = self.font_size.clone();
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
                let (family, url) = FONT_FAMILIES[font_family_index.get() as usize];
                let (_, ref weight) = FONT_WEIGHTS[font_weight_index.get() as usize];
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
                        fill: Property::new(font_color),
                        underline: Property::new(false),
                        align_multiline: Property::new(h_align.clone()),
                        align_vertical: Property::new(v_align.clone()),
                        align_horizontal: Property::new(h_align.clone()),
                    };
                    Self::update_textstyle(&data, &text_style, &cctx);
                }
                external_change.set(false);
                true
            },
            &deps,
        ));
    }

    fn update_textstyle(
        data: &Property<PropertyEditorData>,
        text_style: &TextStyle,
        ctx: &NodeContext,
    ) {
        // TODO use this, and change out color serialization/deserialization
        // let var_str = pax_designtime::serde_pax::se::to_pax(&style);
        // let var_str = var_str.unwrap();
        let var_str = format!(
            "{{
        font: {}
        font_size: {}
        fill: {}
        align_vertical: TextAlignVertical::{:?},
        align_horizontal: TextAlignHorizontal::{:?},
        align_multiline: TextAlignHorizontal::{:?},
        underline: false
        }}",
            {
                let font = text_style.font.get();
                match font {
                    Font::Web(family, url, style, weight) => {
                        format!(
                            "Font::Web({:?}, {:?}, FontStyle::{:?}, FontWeight::{:?})",
                            family, url, style, weight
                        )
                    }
                }
            },
            match text_style.font_size.get() {
                Size::Pixels(px) => format!("{}px", px),
                Size::Percent(perc) => format!("{}%", perc),
                Size::Combined(px, perc) => format!("{}px + {}%", px, perc),
            },
            {
                let rgba = text_style.fill.get().to_rgba_0_1();
                format!(
                    "rgba({}, {}, {}, {})",
                    (rgba[0] * 255.0) as u8,
                    (rgba[1] * 255.0) as u8,
                    (rgba[2] * 255.0) as u8,
                    (rgba[3] * 255.0) as u8
                )
            },
            text_style.align_vertical.get(),
            text_style.align_horizontal.get(),
            text_style.align_horizontal.get(),
        );
        if let Err(e) = data.get().set_value(ctx, &var_str) {
            log::warn!("failed to write textstyle: {e}");
        }
    }

    pub fn pre_render(&mut self, _ctx: &NodeContext) {
        // trigger dirty gets, if deps have changed
        self.property_listeners.get();
    }

    pub fn size_change(&mut self, _ctx: &NodeContext, event: Event<TextboxChange>) {
        self.font_size.set(event.text.clone());
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

    pub fn set_auto_size(&mut self, ctx: &NodeContext, _event: Event<Click>) {
        let data = self.data.get();
        let uid = UniqueTemplateNodeIdentifier::build(data.stid, data.snid);
        model::perform_action(
            &SetNodeLayout {
                id: &uid,
                node_layout: &NodeLayoutSettings::WithProperties::<Generic>(LayoutProperties {
                    width: Some(Size::default()),
                    height: Some(Size::default()),
                    ..Default::default()
                }),
            },
            ctx,
        );
    }
}
