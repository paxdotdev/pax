#![allow(unused)]
pub const NUMERIC: &str = "Numeric";
pub const SIZE: &str = "Size";
pub const ROTATION: &str = "Rotation";
pub const STRING_BOX: &str = "StringBox";
pub const DEGREES: &str = "Degrees";
pub const RADIANS: &str = "Radians";
pub const PIXELS: &str = "Pixels";
pub const PERCENT: &str = "Percent";
pub const INTEGER: &str = "Integer";
pub const FLOAT: &str = "Float";
pub const TRUE: &str = "true";
pub const COLOR: &str = "Color";
pub const COLOR_CHANNEL: &str = "ColorChannel";

pub const TYPE_ID_IF: &str = "IF";
pub const TYPE_ID_REPEAT: &str = "REPEAT";
pub const TYPE_ID_SLOT: &str = "SLOT";
pub const TYPE_ID_COMMENT: &str = "COMMENT";

pub const COMMON_PROPERTIES: [&str; 13] = [
    "id",
    "x",
    "y",
    "scale_x",
    "scale_y",
    "skew_x",
    "skew_y",
    "anchor_x",
    "anchor_y",
    "rotate",
    "transform",
    "width",
    "height",
];

pub fn is_intoable_downstream_type(type_to_check: &str) -> bool {
    blessed_intoable_downstream_types
        .iter()
        .any(|bidt| type_to_check.contains(*bidt))
}

// Only when parsing values for one of the types in this slice
// will we look ahead and parse for an IntoableLiteral value.
const blessed_intoable_downstream_types: [&'static str; 5] = [
    "pax_engine::api::Size",
    "pax_engine::api::Rotation",
    "pax_engine::api::ColorChannel",
    "pax_std::types::Stroke",
    "pax_std::types::Fill",
];

pub const COMMON_PROPERTIES_TYPE: [(&str, &str); 13] = [
    ("id", "String"),
    ("x", "pax_engine::api::Size"),
    ("y", "pax_engine::api::Size"),
    ("scale_x", "pax_engine::api::Size"),
    ("scale_y", "pax_engine::api::Size"),
    ("skew_x", "pax_engine::api::Numeric"),
    ("skew_y", "pax_engine::api::Numeric"),
    ("anchor_x", "pax_engine::api::Size"),
    ("anchor_y", "pax_engine::api::Size"),
    ("rotate", "pax_engine::api::Rotation"),
    ("transform", "pax_engine::api::Transform2D"),
    ("width", "pax_engine::api::Size"),
    ("height", "pax_engine::api::Size"),
];
