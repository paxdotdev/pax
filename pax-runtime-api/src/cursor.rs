#[derive(Debug, Clone, PartialEq)]
pub enum CursorStyle {
    // General
    Auto,
    Default,
    None,

    // Links & Status
    ContextMenu,
    Help,
    Pointer,
    Progress,
    Wait,

    // Selection
    Cell,
    Crosshair,
    Text,
    VerticalText,

    // Drag & Drop
    Alias,
    Copy,
    Move,
    NoDrop,
    NotAllowed,

    // Resizing & Scrolling
    AllScroll,
    ColResize,
    RowResize,
    NResize,
    EResize,
    SResize,
    WResize,
    NeResize,
    NwResize,
    SeResize,
    SwResize,
    EwResize,
    NsResize,
    NeswResize,
    NwseResize,

    // Zooming
    ZoomIn,
    ZoomOut,

    // Grabbing
    Grab,
    Grabbing,

    // Custom
    Custom(String),
}

impl std::fmt::Display for CursorStyle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // NOTE: These should match the valid css cursor names
        match self {
            CursorStyle::Auto => write!(f, "auto"),
            CursorStyle::Default => write!(f, "default"),
            CursorStyle::None => write!(f, "none"),
            CursorStyle::ContextMenu => write!(f, "context-menu"),
            CursorStyle::Help => write!(f, "help"),
            CursorStyle::Pointer => write!(f, "pointer"),
            CursorStyle::Progress => write!(f, "progress"),
            CursorStyle::Wait => write!(f, "wait"),
            CursorStyle::Cell => write!(f, "cell"),
            CursorStyle::Crosshair => write!(f, "crosshair"),
            CursorStyle::Text => write!(f, "text"),
            CursorStyle::VerticalText => write!(f, "vertical-text"),
            CursorStyle::Alias => write!(f, "alias"),
            CursorStyle::Copy => write!(f, "copy"),
            CursorStyle::Move => write!(f, "move"),
            CursorStyle::NoDrop => write!(f, "no-drop"),
            CursorStyle::NotAllowed => write!(f, "not-allowed"),
            CursorStyle::AllScroll => write!(f, "all-scroll"),
            CursorStyle::ColResize => write!(f, "col-resize"),
            CursorStyle::RowResize => write!(f, "row-resize"),
            CursorStyle::NResize => write!(f, "n-resize"),
            CursorStyle::EResize => write!(f, "e-resize"),
            CursorStyle::SResize => write!(f, "s-resize"),
            CursorStyle::WResize => write!(f, "w-resize"),
            CursorStyle::NeResize => write!(f, "ne-resize"),
            CursorStyle::NwResize => write!(f, "nw-resize"),
            CursorStyle::SeResize => write!(f, "se-resize"),
            CursorStyle::SwResize => write!(f, "sw-resize"),
            CursorStyle::EwResize => write!(f, "ew-resize"),
            CursorStyle::NsResize => write!(f, "ns-resize"),
            CursorStyle::NeswResize => write!(f, "nesw-resize"),
            CursorStyle::NwseResize => write!(f, "nwse-resize"),
            CursorStyle::ZoomIn => write!(f, "zoom-in"),
            CursorStyle::ZoomOut => write!(f, "zoom-out"),
            CursorStyle::Grab => write!(f, "grab"),
            CursorStyle::Grabbing => write!(f, "grabbing"),
            CursorStyle::Custom(url) => write!(f, "{}", url),
        }
    }
}
