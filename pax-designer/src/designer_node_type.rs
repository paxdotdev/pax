use pax_engine::pax_manifest::{PaxType, TypeId};

#[derive(PartialEq)]
pub enum DesignerNodeType {
    Frame,
    Group,
    Link,
    Ellipse,
    Text,
    Stacker,
    Rectangle,
    Path,
    Component { name: String, import_path: String },
    Textbox,
    Checkbox,
    Scroller,
    Button,
    Image,
    Slider,
    Dropdown,
    RadioSet,
    If,
    For,
    Unregistered(TypeId),
}

pub struct DesignerNodeTypeData {
    pub name: String,
    pub image_path: String,
    pub is_container: bool,
    /// This is true for stacker and other types that place their children in
    /// a custom configuration. (not nessesarily true for all containers using
    /// slots). Ie scrollers slots allways take upp the full space, and so is not
    /// characterized as a slot container
    pub is_slot_container: bool,
    pub type_id: TypeId,
}

impl DesignerNodeType {
    pub fn from_type_id(type_id: TypeId) -> Self {
        match type_id.get_pax_type() {
            PaxType::If => DesignerNodeType::If,
            PaxType::Repeat => DesignerNodeType::For,
            _ => {
                let Some(import_path) = type_id.import_path() else {
                    return DesignerNodeType::Unregistered(type_id);
                };
                // TODO make this and  the metadata method use the same constants, or maybe even a signle
                // Vec<(TypeId, DesignerNodeType)> that can be searched in either direction.
                match import_path.trim_start_matches("pax_std::") {
                    "core::group::Group" => DesignerNodeType::Group,
                    "core::link::Link" => DesignerNodeType::Link,
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
                    _ => {
                        let (_, name) = import_path.rsplit_once("::").unwrap_or(("", &import_path));
                        DesignerNodeType::Component {
                            name: name.to_string(),
                            import_path,
                        }
                    }
                }
            }
        }
    }

    pub fn metadata(&self) -> DesignerNodeTypeData {
        let (name, img_path_suffix, type_id, is_container) = match self {
            DesignerNodeType::Frame => (
                "Frame",
                "frame",
                TypeId::build_singleton("pax_std::core::frame::Frame", None),
                true,
            ),
            DesignerNodeType::Group => (
                "Group",
                "group",
                TypeId::build_singleton("pax_std::core::group::Group", None),
                true,
            ),
            // TODO custom image
            DesignerNodeType::Link => (
                "Link",
                "group",
                TypeId::build_singleton("pax_std::core::link::Link", None),
                true,
            ),
            DesignerNodeType::Ellipse => (
                "Ellipse",
                "ellipse",
                TypeId::build_singleton("pax_std::drawing::ellipse::Ellipse", None),
                false,
            ),
            DesignerNodeType::Text => (
                "Text",
                "text",
                TypeId::build_singleton("pax_std::core::text::Text", None),
                false,
            ),
            DesignerNodeType::Stacker => (
                "Stacker",
                "stacker",
                TypeId::build_singleton("pax_std::layout::stacker::Stacker", None),
                true,
            ),
            DesignerNodeType::Rectangle => (
                "Rectangle",
                "rectangle",
                TypeId::build_singleton("pax_std::drawing::rectangle::Rectangle", None),
                false,
            ),
            DesignerNodeType::Path => (
                "Path",
                "path",
                TypeId::build_singleton("pax_std::drawing::path::Path", None),
                false,
            ),
            DesignerNodeType::Component { name, import_path } => (
                name.as_str(),
                "component",
                TypeId::build_singleton(import_path, None),
                false,
            ),
            DesignerNodeType::Textbox => (
                "Textbox",
                "textbox",
                TypeId::build_singleton("pax_std::forms::textbox::Textbox", None),
                false,
            ),
            DesignerNodeType::Checkbox => (
                "Checkbox",
                "checkbox",
                TypeId::build_singleton("pax_std::forms::checkbox::Checkbox", None),
                false,
            ),
            DesignerNodeType::Scroller => (
                "Scroller",
                "scroller",
                TypeId::build_singleton("pax_std::core::scroller::Scroller", None),
                true,
            ),
            DesignerNodeType::Button => (
                "Button",
                "button",
                TypeId::build_singleton("pax_std::forms::button::Button", None),
                false,
            ),
            DesignerNodeType::Image => (
                "Image",
                "image",
                TypeId::build_singleton("pax_std::core::image::Image", None),
                false,
            ),
            DesignerNodeType::Slider => (
                "Slider",
                "slider",
                TypeId::build_singleton("pax_std::forms::slider::Slider", None),
                false,
            ),
            DesignerNodeType::Dropdown => (
                "Dropdown",
                "dropdown",
                TypeId::build_singleton("pax_std::forms::dropdown::Dropdown", None),
                false,
            ),
            DesignerNodeType::If => ("If", "if", TypeId::build_if(), true),
            DesignerNodeType::For => ("For", "for", TypeId::build_repeat(), true),
            DesignerNodeType::Unregistered(type_id) => {
                ("[Unregistered Type]", "component", type_id.clone(), false)
            }
            // TODO add custom image
            DesignerNodeType::RadioSet => (
                "Radio Set",
                "component",
                TypeId::build_singleton("pax_std::forms::radio_set::RadioSet", None),
                false,
            ),
        };

        // move to match statement above if more types need this specified
        let is_slot_container = self == &DesignerNodeType::Stacker;

        DesignerNodeTypeData {
            name: name.to_owned(),
            image_path: format!("assets/icons/icon-{}.png", img_path_suffix),
            is_container,
            is_slot_container,
            type_id,
        }
    }
}
