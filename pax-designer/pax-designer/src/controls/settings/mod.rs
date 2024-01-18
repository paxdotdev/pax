use std::collections::HashMap;

use pax_lang::api::*;
use pax_lang::*;
use pax_std::components::Stacker;
use pax_std::components::*;
use pax_std::primitives::*;
use pax_std::types::text::*;
use pax_std::types::*;

pub mod property_editor;
use property_editor::PropertyEditor;

#[derive(Pax)]
#[file("controls/settings/settings.pax")]
pub struct Settings {
    pub component_selected: Property<bool>,
    pub selected_component_name: Property<String>,

    // common props
    pub pos_x: Property<StringBox>,
    pub pos_y: Property<StringBox>,
    pub size_width: Property<StringBox>,
    pub size_height: Property<StringBox>,
    pub rotation_z: Property<StringBox>,
    pub scale_x: Property<StringBox>,
    pub scale_y: Property<StringBox>,
    pub anchor_x: Property<StringBox>,
    pub anchor_y: Property<StringBox>,
    pub skew_x: Property<StringBox>,
    pub skew_y: Property<StringBox>,

    // custom props
    pub custom_props: Property<Vec<PropertyDef>>,

    pub run_id: Property<u32>,
    pub timer: Property<u32>,
}

#[derive(Pax)]
#[custom(Imports)]
pub struct PropertyDef {
    pub name: StringBox,
    pub definition: StringBox,
}

impl Settings {
    pub fn on_mount(&mut self, _ctx: &NodeContext) {
        self.deselect();
    }

    pub fn set_none(&mut self, _ctx: &NodeContext, args: ArgsButtonClick) {
        self.deselect();
    }

    pub fn deselect(&mut self) {
        self.selected_component_name
            .set("Select Component".to_owned());
        self.component_selected.set(false);
    }

    pub fn pre_render(&mut self, _ctx: &NodeContext) {
        if *self.timer.get() > 0 {
            if *self.timer.get() == 1 {
                match *self.run_id.get() {
                    1 => self.set_object1(),
                    2 => self.set_object2(),
                    _ => (),
                }
            }
            self.timer.set(self.timer.get() - 1);
        }
    }

    pub fn set_object2_defered(&mut self, _ctx: &NodeContext, args: ArgsButtonClick) {
        self.run_id.set(2);
        self.timer.set(2);
        self.custom_props.set(vec![]);
    }

    pub fn set_object1_defered(&mut self, _ctx: &NodeContext, args: ArgsButtonClick) {
        self.run_id.set(1);
        self.timer.set(2);
        self.custom_props.set(vec![]);
    }

    pub fn set_object1(&mut self) {
        self.component_selected.set(true);
        self.selected_component_name.set("ELLIPSE 1".to_owned());
        self.pos_x.set(StringBox::from("20".to_owned()));
        self.pos_y.set(StringBox::from("40".to_owned()));
        self.size_width.set(StringBox::from("1000".to_owned()));
        self.size_height.set(StringBox::from("1000000".to_owned()));
        self.rotation_z.set(StringBox::from("0.0".to_owned()));
        self.scale_x.set(StringBox::from("1.0".to_owned()));
        self.scale_y.set(StringBox::from("2.0".to_owned()));
        self.anchor_x.set(StringBox::from("0%".to_owned()));
        self.anchor_y.set(StringBox::from("0%".to_owned()));
        self.skew_x.set(StringBox::from("0.0".to_owned()));
        self.skew_y.set(StringBox::from("1.0".to_owned()));
        // OBS: if two objects happen to have the same number of props when selection updates, it doesn't change
        self.custom_props.set(vec![
            PropertyDef {
                name: StringBox::from("Stroke".to_owned()),
                definition: StringBox::from("Color::rgba(0.0, 1.0, 0.0, 1.0)".to_owned()),
            },
            PropertyDef {
                name: StringBox::from("Fill".to_owned()),
                definition: StringBox::from("Color::rgba(1.0, 0.0, 0.0, 1.0)".to_owned()),
            },
        ]);
    }

    pub fn set_object2(&mut self) {
        self.component_selected.set(true);
        self.selected_component_name.set("TEXT 2".to_owned());
        self.pos_x.set(StringBox::from("0.7".to_owned()));
        self.pos_y.set(StringBox::from("100.0".to_owned()));
        self.size_width.set(StringBox::from("500".to_owned()));
        self.size_height.set(StringBox::from("300".to_owned()));
        self.rotation_z.set(StringBox::from("30".to_owned()));
        self.scale_x.set(StringBox::from("1.0".to_owned()));
        self.scale_y.set(StringBox::from("1.0".to_owned()));
        self.anchor_x.set(StringBox::from("50%".to_owned()));
        self.anchor_y.set(StringBox::from("30%".to_owned()));
        self.skew_x.set(StringBox::from("0.0".to_owned()));
        self.skew_y.set(StringBox::from("0.0".to_owned()));
        self.custom_props.set(vec![
            PropertyDef {
                name: StringBox::from("Text".to_owned()),
                definition: StringBox::from("This is some example text".to_owned()),
            },
            PropertyDef {
                name: StringBox::from("Style".to_owned()),
                definition: StringBox::from("{ fill: ...}".to_owned()),
            },
        ]);
    }
}
