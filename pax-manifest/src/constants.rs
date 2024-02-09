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


pub const TYPE_ID_IF: &str = "IF";
pub const TYPE_ID_REPEAT: &str = "REPEAT";
pub const TYPE_ID_SLOT: &str = "SLOT";
pub const TYPE_ID_COMMENT: &str = "COMMENT";

pub const COMMON_PROPERTIES: [&str; 13] =  [
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

pub const COMMON_PROPERTIES_TYPE: [(&str, &str); 13] =  [
    ("id", "String"),
    ("x", "Size"),
    ("y", "Size"),
    ("scale_x", "Size"),
    ("scale_y", "Size"),
    ("skew_x", "Numeric"),
    ("skew_y", "Numeric"),
    ("anchor_x", "Size"),
    ("anchor_y", "Size"),
    ("rotate", "Rotation"),
    ("transform", "Transform2D"),
    ("width", "Size"),
    ("height", "Size"),
];