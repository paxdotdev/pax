pub mod common;
pub mod core;
pub mod drawing;
pub mod forms;
pub mod layout;
pub mod media;

pub use common::*;
pub use core::*;
pub use drawing::*;
pub use forms::*;
pub use layout::*;
pub use media::*;

#[cfg(feature = "parser")]
// Registers all pax-std reflectable types with designtime parsing.
pub fn extend_designtime_parsing_context_with_all_pax_std_types(
    mut ctx: pax_manifest::parsing::ParsingContext,
) -> pax_manifest::parsing::ParsingContext {
    use pax_manifest::parsing::Reflectable;

    macro_rules! parse_reflectables {
        ($($ty:ty),* $(,)?) => {
            $(
                let (next_ctx, _) = <$ty as Reflectable>::parse_to_manifest(ctx);
                ctx = next_ctx;
            )*
        };
    }

    // Seed the parser-time manifest with the full public pax-std surface so
    // designtime template hot-reloads can introduce standard-library tags that
    // were not present in the original userland tree.
    parse_reflectables!(
        BlankComponent,
        ComboBox,
        NewItem,
        ComboBoxListItem,
        ListItemData,
        ComboBoxItemClickEvent,
        EventBlocker,
        Frame,
        Group,
        ImportSettings,
        Link,
        Router,
        Route,
        Target,
        NativeImage,
        Scroller,
        ScrollerHost,
        Text,
        TextStyle,
        Font,
        FontStyle,
        FontWeight,
        TextAlignHorizontal,
        TextAlignVertical,
        Tooltip,
        YoutubeVideo,
        Ellipse,
        Image,
        ImageSource,
        ImageFit,
        Line,
        Path,
        PathPoint,
        PathLine,
        PathClose,
        PathCurve,
        Rectangle,
        RectangleCornerRadii,
        Button,
        Checkbox,
        ConfirmationDialog,
        Dropdown,
        RadioList,
        Slider,
        Tabs,
        Textbox,
        Toast,
        Carousel,
        CarouselAxis,
        CarouselCell,
        Resizable,
        ResizableDirection,
        Section,
        Stacker,
        StackerCell,
        StackerDirection,
        ContainerExitMode,
        ContainerReflowTransition,
        ContainerReflowCurve,
        ContainerReflowTransitionKind,
        Table,
        Row,
        Col,
        Span,
        Cell,
    );

    #[cfg(feature = "designtime")]
    {
        let (next_ctx, _) =
            <crate::core::inline_frame::InlineFrame as Reflectable>::parse_to_manifest(ctx);
        ctx = next_ctx;
    }

    ctx
}
