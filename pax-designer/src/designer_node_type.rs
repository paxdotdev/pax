use pax_engine::pax_manifest::{PaxType, TypeId};

use crate::controls::tree::DesignerNodeType;

pub struct DesignerNodeTypeData {
    pub name: String,
    pub image_path: String,
    pub is_container: bool,
    /// This is true for stacker and other types that place their children in
    /// a custom configuration. (not nessesarily true for all containers using
    /// slots). Ie scrollers slots allways take upp the full space, and so is not
    /// characterized as a slot container
    pub is_slot_container: bool,
}

impl DesignerNodeType {
    pub fn from_type_id(type_id: TypeId) -> Self {
        match type_id.get_pax_type() {
            PaxType::If => DesignerNodeType::If,
            PaxType::Repeat => DesignerNodeType::For,
            _ => {
                let Some(import_path) = type_id.import_path() else {
                    return DesignerNodeType::Component(format!("{}", type_id.get_pax_type()));
                };
                match import_path.trim_start_matches("pax_std::") {
                    "core::group::Group" => DesignerNodeType::Group,
                    "core::frame::Frame" => DesignerNodeType::Frame,
                    "drawing::ellipse::Ellipse" => DesignerNodeType::Ellipse,
                    "core::text::Text" => DesignerNodeType::Text,
                    "layout::stacker::Stacker" => DesignerNodeType::Stacker,
                    "drawing::rectangle::Rectangle" => DesignerNodeType::Rectangle,
                    "drawing::path::Path" => DesignerNodeType::Path,
                    "forms::textbox::Textbox" => DesignerNodeType::Textbox,
                    "forms::checkbox::Checkbox" => DesignerNodeType::Checkbox,
                    "core::scroller::Scroller" => DesignerNodeType::Scroller,
                    "forms::button::Button" => DesignerNodeType::Button,
                    "core::image::Image" => DesignerNodeType::Image,
                    "forms::slider::Slider" => DesignerNodeType::Slider,
                    "forms::dropdown::Dropdown" => DesignerNodeType::Dropdown,
                    _ => DesignerNodeType::Component(format!("{}", type_id.get_pax_type())),
                }
            }
        }
    }

    pub fn metadata(&self) -> DesignerNodeTypeData {
        let (name, img_path_suffix, is_container) = match self {
            DesignerNodeType::Frame => ("Frame", "frame", true),
            DesignerNodeType::Group => ("Group", "group", true),
            DesignerNodeType::Ellipse => ("Ellipse", "ellipse", false),
            DesignerNodeType::Text => ("Text", "text", false),
            DesignerNodeType::Stacker => ("Stacker", "stacker", true),
            DesignerNodeType::Rectangle => ("Rectangle", "rectangle", false),
            DesignerNodeType::Path => ("Path", "path", false),
            DesignerNodeType::Component(name) => (name.as_str(), "component", false),
            DesignerNodeType::Textbox => ("Textbox", "textbox", false),
            DesignerNodeType::Checkbox => ("Checkbox", "checkbox", false),
            DesignerNodeType::Scroller => ("Scroller", "scroller", true),
            DesignerNodeType::Button => ("Button", "button", false),
            DesignerNodeType::Image => ("Image", "image", false),
            DesignerNodeType::Slider => ("Slider", "slider", false),
            DesignerNodeType::Dropdown => ("Dropdown", "dropdown", false),
            DesignerNodeType::If => ("If", "if", true),
            DesignerNodeType::For => ("For", "for", true),
        };

        // move to match statement above if more types need this specified
        let is_slot_container = self == &DesignerNodeType::Stacker;

        DesignerNodeTypeData {
            name: name.to_owned(),
            image_path: format!("assets/icons/icon-{}.png", img_path_suffix),
            is_container,
            is_slot_container,
        }
    }
}
