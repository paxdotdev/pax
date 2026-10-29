#![allow(unused_imports)]

use crate::*;
use pax_engine::api::LocalProperty;
use pax_engine::api::{
    cursor::CursorStyle, ButtonClick, Click, Color, Duration, EasingCurve, Event, MouseDown,
    MouseMove, MouseOut, MouseOver, MouseUp,
};
use pax_engine::*;
use pax_runtime::api::NodeContext;

const DESKTOP_DRAWER_WIDTH_PX: f64 = 430.0;
const MIN_PREVIEW_WIDTH_PX: f64 = 320.0;
const MIN_DRAWER_WIDTH_PX: f64 = 280.0;
const RESIZE_MIN_DELTA_PX: f64 = 2.0;
const DRAWER_EASE_MS: u64 = 250;

// Canonical Solarized sRGB values: https://ethanschoonover.com/solarized/.
// Only the neutral roles invert; syntax accents stay the same in both modes.
const CODE_TYPE: &str = "#b58900";
const CODE_NUMBER: &str = "#2aa198";
const CODE_STRING: &str = "#2aa198";
const CODE_PROPERTY: &str = "#268bd2";
const CODE_FUNCTION: &str = "#268bd2";
const CODE_KEYWORD: &str = "#859900";

#[derive(Clone, Copy)]
struct SourcePalette {
    background: &'static str,
    foreground: &'static str,
    comment: &'static str,
}

impl SourcePalette {
    fn for_light_mode(light: bool) -> Self {
        if light {
            Self {
                background: "#fdf6e3", // base3
                foreground: "#657b83", // base00
                comment: "#93a1a1",    // base1
            }
        } else {
            Self {
                background: "#002b36", // base03
                foreground: "#839496", // base0
                comment: "#586e75",    // base01
            }
        }
    }
}

/// A display wrapper for documentation/demo examples.
///
/// `ExampleHost` renders every projected child with `slot()` and offers an
/// optional source drawer driven by an explicit `sources` manifest.
/// The code pane offers Solarized Light (default) and Dark through a moon/sun switch.
/// Theme state is local to the host unless `source_light_mode` is explicitly bound.
/// Set `inline_source_viewer=false` to present a separately mounted
/// [`ExampleSourceViewer`], binding its open, selection, and theme properties.
#[pax]
#[engine_import_path("pax_engine")]
#[file("reference/example_host.pax")]
#[custom(Default)]
pub struct ExampleHost {
    /// Title shown in the source drawer.
    pub title: Property<String>,
    /// Explicit source files related to the projected example subtree.
    pub sources: Property<Vec<ExampleSource>>,
    /// Selected source index for the drawer tabs.
    pub selected_source: Property<usize>,
    /// Whether the source drawer is visible.
    pub drawer_open: Property<bool>,
    /// Use Solarized Light (default) instead of Dark inside the code pane only.
    /// The moon/sun switch edits this value; use `bind:` to share it deliberately.
    pub source_light_mode: Property<bool>,
    /// Present the source viewer beside the preview (default true).
    /// When false, View Source still toggles `drawer_open`, but the caller must
    /// mount an `ExampleSourceViewer` elsewhere, such as above a page Scroller.
    pub inline_source_viewer: Property<bool>,
    // Private source-panel width in pixels, remembered while the drawer closes.
    pub _drawer_width_px: Property<f64>,
    // Private animated source drawer progress, from closed 0.0 to open 1.0.
    pub _drawer_progress: Property<f64>,
    // Private last observed public drawer state, used for external control sync.
    pub _drawer_target_open: Property<bool>,
    // Private drag state for the source divider.
    pub _is_resizing_drawer: Property<bool>,
    // Private responsive mode where the source drawer replaces the preview.
    pub _compact_drawer: Property<bool>,
}

impl Default for ExampleHost {
    fn default() -> Self {
        Self {
            title: Property::new("Example".to_string()),
            sources: Property::new(vec![]),
            selected_source: Property::new(0),
            drawer_open: Property::new(false),
            source_light_mode: Property::new(true),
            inline_source_viewer: Property::new(true),
            _drawer_width_px: Property::new(DESKTOP_DRAWER_WIDTH_PX),
            _drawer_progress: Property::new(0.0),
            _drawer_target_open: Property::new(false),
            _is_resizing_drawer: Property::new(false),
            _compact_drawer: Property::new(false),
        }
    }
}

impl ExampleHost {
    // Derive the source pane from this host's selected index.
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        let drawer_open = self.drawer_open.get();
        self._drawer_target_open.set(drawer_open);
        self._drawer_progress
            .set(if drawer_open { 1.0 } else { 0.0 });
        self.clamp_drawer_to_host(ctx.bounds_self.get().0);
    }

    /// Keeps the animated drawer progress synchronized with external state writes.
    pub fn on_pre_render(&mut self, ctx: &NodeContext) {
        self.clamp_drawer_to_host(ctx.bounds_self.get().0);
        let drawer_open = self.drawer_open.get();
        if drawer_open != self._drawer_target_open.get() {
            self._drawer_target_open.set(drawer_open);
            self.animate_drawer_progress(drawer_open);
        }
    }

    /// Toggles the source drawer.
    pub fn toggle_drawer(&mut self, _ctx: &NodeContext, _event: Event<Click>) {
        let next_open = !self.drawer_open.get();
        self.drawer_open.set(next_open);
        self._drawer_target_open.set(next_open);
        self.animate_drawer_progress(next_open);
    }

    /// Starts dragging the source divider when pressed near the handle.
    pub fn on_resize_mouse_down(&mut self, ctx: &NodeContext, event: Event<MouseDown>) {
        if !self.drawer_open.get() || !self.inline_source_viewer.get() || self._compact_drawer.get()
        {
            return;
        }

        let bounds = ctx.bounds_self.get();
        let divider_px = bounds.0 - self._drawer_width_px.get();
        if (event.mouse.x - divider_px).abs() < 12.0 {
            self._is_resizing_drawer.set(true);
            ctx.set_cursor(CursorStyle::EwResize);
        }
    }

    /// Starts dragging from the explicit source divider hit target.
    pub fn on_split_mouse_down(&mut self, ctx: &NodeContext, _event: Event<MouseDown>) {
        if !self.drawer_open.get() || !self.inline_source_viewer.get() || self._compact_drawer.get()
        {
            return;
        }
        self._is_resizing_drawer.set(true);
        ctx.set_cursor(CursorStyle::EwResize);
    }

    /// Shows the platform resize cursor over the source divider.
    pub fn on_resize_mouse_over(&mut self, ctx: &NodeContext, _event: Event<MouseOver>) {
        ctx.set_cursor(CursorStyle::EwResize);
    }

    /// Restores the cursor after leaving the source divider.
    pub fn on_resize_mouse_out(&mut self, ctx: &NodeContext, _event: Event<MouseOut>) {
        if !self._is_resizing_drawer.get() {
            ctx.set_cursor(CursorStyle::Auto);
        }
    }

    /// Updates the fixed-sum preview/source split while dragging.
    pub fn on_resize_mouse_move(&mut self, ctx: &NodeContext, event: Event<MouseMove>) {
        if !self._is_resizing_drawer.get() {
            return;
        }

        let bounds = ctx.bounds_self.get();
        if bounds.0 < MIN_PREVIEW_WIDTH_PX + MIN_DRAWER_WIDTH_PX {
            self._drawer_width_px.set_if_neq(bounds.0.max(0.0));
            return;
        }
        let max_drawer_width = (bounds.0 - MIN_PREVIEW_WIDTH_PX).max(MIN_DRAWER_WIDTH_PX);
        let drawer_width = (bounds.0 - event.mouse.x).clamp(MIN_DRAWER_WIDTH_PX, max_drawer_width);
        if (drawer_width - self._drawer_width_px.get()).abs() >= RESIZE_MIN_DELTA_PX {
            self._drawer_width_px.set(drawer_width);
        }
    }

    /// Ends source divider dragging.
    pub fn on_resize_mouse_up(&mut self, ctx: &NodeContext, _event: Event<MouseUp>) {
        self._is_resizing_drawer.set(false);
        ctx.set_cursor(CursorStyle::Auto);
    }

    fn animate_drawer_progress(&self, open: bool) {
        let target = if open { 1.0 } else { 0.0 };
        self._drawer_progress.local().ease_to(
            target,
            Duration::Milliseconds(DRAWER_EASE_MS.into()),
            EasingCurve::InQuad,
        );
    }

    fn clamp_drawer_to_host(&self, host_width: f64) {
        let Some((compact_drawer, next_width)) =
            clamped_drawer_layout(host_width, self._drawer_width_px.get())
        else {
            return;
        };
        self._compact_drawer.set_if_neq(compact_drawer);
        self._drawer_width_px.set_if_neq(next_width);
    }
}

/// Reusable source panel with fixed black/white chrome and a Solarized code pane.
///
/// It fills the bounds supplied by its caller, not the global viewport. For a
/// full-screen presentation, mount it above (outside) the page Scroller and bind
/// `open`, `selected_source`, and `source_light_mode` to the owning state. The
/// caller controls mounting; the close control sets `open=false`.
#[pax]
#[engine_import_path("pax_engine")]
#[file("reference/example_source_viewer.pax")]
#[has_helpers]
#[custom(Default)]
pub struct ExampleSourceViewer {
    /// Wrapping title beside the controls; its measured height positions the source tabs.
    pub title: Property<String>,
    /// Source files offered by this viewer.
    pub sources: Property<Vec<ExampleSource>>,
    /// Selected source index; bind to preserve it across viewer remounts.
    pub selected_source: Property<usize>,
    /// Close control writes false; the caller decides how to dismiss the viewer.
    pub open: Property<bool>,
    /// True (default) selects Solarized Light; false selects Dark. Chrome stays black/white.
    pub source_light_mode: Property<bool>,
    pub _title_height: Property<f64>,
    pub _active_source_markup: Property<String>,
}

impl Default for ExampleSourceViewer {
    fn default() -> Self {
        Self {
            title: Property::default(),
            sources: Property::default(),
            selected_source: Property::default(),
            open: Property::default(),
            source_light_mode: Property::new(true),
            _title_height: Property::default(),
            _active_source_markup: Property::default(),
        }
    }
}

impl ExampleSourceViewer {
    pub fn on_mount(&mut self, _ctx: &NodeContext) {
        let sources = self.sources.local();
        let light = self.source_light_mode.local();
        let selected = self.selected_source.local();
        let deps = [sources.untyped(), light.untyped(), selected.untyped()];
        self._active_source_markup
            .replace_with(LocalProperty::computed(
                // Hidden tabs can contain large generated geometry files. Do
                // not highlight those merely to open or recolor this panel.
                move || selected_source_markup(&sources.get(), selected.get(), light.get()),
                &deps,
            ));
    }

    pub fn measure_title(&mut self, ctx: &NodeContext) {
        let Some(node) = ctx.expanded_node.upgrade() else {
            return;
        };
        let measured =
            pax_runtime::node_interface::NodeInterface::from(node).measured_size_property();
        let deps = [measured.untyped()];
        // Use intrinsic text measurement, not provisional parent bounds, so
        // wrapping and late font loads reflow the panel without a size cycle.
        self._title_height.replace_with(LocalProperty::computed(
            move || measured.get().map_or(0.0, |(_, height)| height.ceil()),
            &deps,
        ));
    }

    pub fn close(&mut self, _ctx: &NodeContext, _event: Event<ButtonClick>) {
        self.open.set(false);
    }

    pub fn toggle_theme(&mut self, _ctx: &NodeContext, _event: Event<ButtonClick>) {
        self.source_light_mode.set(!self.source_light_mode.get());
    }
}

#[helpers]
impl ExampleSourceViewer {
    /// Code-pane background for a Solarized mode.
    pub fn source_background(light: bool) -> Color {
        Color::from_hex(&SourcePalette::for_light_mode(light).background[1..])
    }

    /// Code-pane body text for a Solarized mode.
    pub fn source_foreground(light: bool) -> Color {
        Color::from_hex(&SourcePalette::for_light_mode(light).foreground[1..])
    }
}

fn clamped_drawer_layout(host_width: f64, current_width: f64) -> Option<(bool, f64)> {
    if host_width <= 0.0 {
        return None;
    }
    let compact_drawer = host_width < MIN_PREVIEW_WIDTH_PX + MIN_DRAWER_WIDTH_PX;
    let width = if compact_drawer {
        host_width
    } else {
        current_width.clamp(MIN_DRAWER_WIDTH_PX, host_width - MIN_PREVIEW_WIDTH_PX)
    };
    Some((compact_drawer, width))
}

/// One source file shown by `ExampleHost`.
#[pax]
#[engine_import_path("pax_engine")]
pub struct ExampleSource {
    /// File label shown in drawer tabs.
    pub label: String,
    /// Language label shown in the drawer.
    pub language: String,
    /// Source code contents.
    pub code: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compact_drawer_replaces_preview_at_narrow_widths() {
        assert_eq!(clamped_drawer_layout(390.0, 430.0), Some((true, 390.0)));
    }

    #[test]
    fn wide_drawer_preserves_preview_and_clamps_to_available_space() {
        assert_eq!(clamped_drawer_layout(1_280.0, 430.0), Some((false, 430.0)));
        assert_eq!(
            clamped_drawer_layout(1_280.0, 1_100.0),
            Some((false, 960.0))
        );
    }

    #[test]
    fn source_palette_distinguishes_pax_and_rust_tokens() {
        let cases = [
            (
                "pax",
                "<Text width=24px text=\"Hello\" />",
                vec![
                    (CODE_TYPE, "Text"),
                    (CODE_PROPERTY, "width"),
                    (CODE_NUMBER, "24"),
                    (CODE_STRING, "&quot;Hello&quot;"),
                ],
            ),
            (
                "rust",
                "pub fn update(value: f64) { log!(value); } // note",
                vec![
                    (CODE_KEYWORD, "pub"),
                    (CODE_FUNCTION, "update"),
                    (CODE_FUNCTION, "log"),
                    (CODE_PROPERTY, "value"),
                ],
            ),
        ];
        for (language, code, tokens) in cases {
            let source = ExampleSource {
                label: "test".into(),
                language: language.into(),
                code: code.into(),
            };
            for light in [false, true] {
                let markup = highlighted_code_markup(&source, light);
                for (color, token) in &tokens {
                    assert!(
                        markup.contains(&format!("<span style=\"color:{color}\">{token}</span>"))
                    );
                }
                if language == "rust" {
                    let comment = SourcePalette::for_light_mode(light).comment;
                    assert!(
                        markup.contains(&format!("<span style=\"color:{comment}\">// note</span>"))
                    );
                }
            }
        }
    }

    #[test]
    fn highlighted_source_preserves_escaping_and_plain_identifiers() {
        for light in [false, true] {
            let mut markup = String::new();
            highlight_line(
                "value == other && path::call(\"<é & >\")",
                "rust",
                SourcePalette::for_light_mode(light),
                &mut markup,
            );
            assert!(markup.starts_with("value "));
            assert!(markup.contains(" path<span"));
            assert!(markup.contains("&lt;é &amp; &gt;"));
            assert!(!markup.contains(&format!("color:{CODE_PROPERTY}\">value</span>")));
        }
    }

    #[test]
    fn solarized_roles_use_canonical_values_without_brightening() {
        // Solarized intentionally uses selective contrast, including muted
        // comments; it is not the previous all-tokens-4.5:1 high-contrast palette.
        let dark = SourcePalette::for_light_mode(false);
        let light = SourcePalette::for_light_mode(true);
        assert_eq!(dark.background, "#002b36");
        assert_eq!(light.background, "#fdf6e3");
        assert_eq!((dark.foreground, dark.comment), ("#839496", "#586e75"));
        assert_eq!((light.foreground, light.comment), ("#657b83", "#93a1a1"));
        assert_eq!(
            (CODE_TYPE, CODE_STRING, CODE_KEYWORD, CODE_FUNCTION),
            ("#b58900", "#2aa198", "#859900", "#268bd2")
        );
        for mode in [false, true] {
            let palette = SourcePalette::for_light_mode(mode);
            assert_eq!(
                ExampleSourceViewer::source_background(mode),
                Color::from_hex(&palette.background[1..])
            );
            assert_eq!(
                ExampleSourceViewer::source_foreground(mode),
                Color::from_hex(&palette.foreground[1..])
            );
        }
    }

    #[test]
    fn empty_and_multiple_source_manifests_follow_the_selected_theme() {
        let sources = vec![
            fallback_source(),
            ExampleSource {
                label: "hello.rs".into(),
                language: "rust".into(),
                code: "let x = 1; // hello".into(),
            },
        ];
        for mode in [false, true] {
            let foreground = SourcePalette::for_light_mode(mode).foreground;
            for manifest in [&[][..], sources.as_slice()] {
                for selected in [0, 1, usize::MAX] {
                    let markup = selected_source_markup(manifest, selected, mode);
                    assert!(markup.contains(&format!("color:{foreground};")));
                    let expected = manifest
                        .get(selected.min(manifest.len().saturating_sub(1)))
                        .cloned()
                        .unwrap_or_else(fallback_source);
                    assert_eq!(markup, highlighted_code_markup(&expected, mode));
                }
            }
        }
    }

    #[test]
    fn theme_state_defaults_light_and_does_not_leak_between_hosts_or_change_tabs() {
        let first = ExampleHost::default();
        let sibling = ExampleHost::default();
        assert!(first.source_light_mode.get());
        assert!(ExampleSourceViewer::default().source_light_mode.get());
        first.selected_source.set(3);
        first.source_light_mode.set(false);
        assert!(sibling.source_light_mode.get());
        assert_eq!(first.selected_source.get(), 3);
        first.source_light_mode.set(true);
        assert_eq!(first.selected_source.get(), 3);
    }

    #[test]
    fn explicitly_bound_theme_is_shared_without_sharing_selection_or_open_state() {
        let page_theme = Property::new(true);
        let first = ExampleHost {
            source_light_mode: page_theme.clone(),
            ..Default::default()
        };
        let sibling = ExampleHost {
            source_light_mode: page_theme.clone(),
            ..Default::default()
        };
        let mobile = ExampleSourceViewer {
            source_light_mode: page_theme.clone(),
            ..Default::default()
        };
        first.selected_source.set(2);
        first.drawer_open.set(true);
        first.source_light_mode.set(false);
        assert!(!page_theme.get());
        assert!(!sibling.source_light_mode.get());
        assert!(!mobile.source_light_mode.get());
        mobile.source_light_mode.set(true);
        assert!(first.source_light_mode.get());
        assert!(sibling.source_light_mode.get());
        assert_eq!(first.selected_source.get(), 2);
        assert_eq!(sibling.selected_source.get(), 0);
        assert!(first.drawer_open.get());
        assert!(!sibling.drawer_open.get());
    }
}

// Internal source tab used by `ExampleHost`.
#[pax]
#[engine_import_path("pax_engine")]
#[inlined(
<Group width=100% height=100% @click=self.select_source>
    <Text class="example_host_tab_label" x=50% y=50% width={100% - 16px} height=18px text={source.label} selectable=false
        wrap=false clip=true/>
    if self.index == self.selected {
        <Path class="example_host_tab_active" width=100% height=100%/>
    }
    if self.index != self.selected {
        <Path class="example_host_tab_idle" width=100% height=100%/>
    }
</Group>

@settings {
    .example_host_tab_label {
        style: TextStyle {
            font: Font::Web(
                "Manrope",
                "/assets/fonts/Manrope.ttf",
                FontStyle::Normal,
                FontWeight::Medium)
            font_size: 11px
            fill: {self.index == self.selected ? BLACK : WHITE}
            align_vertical: TextAlignVertical::Center
            align_horizontal: TextAlignHorizontal::Center
            align_multiline: TextAlignHorizontal::Center
            underline: false
        }
    }
    .example_host_tab_active {
        fill: WHITE
        elements: [
            PathElement::Point(0%, 100%),
            PathElement::Line,
            PathElement::Point(2.5%, 17%),
            PathElement::Quadratic(2.5%, 0%),
            PathElement::Point(5%, 0%),
            PathElement::Line,
            PathElement::Point(95%, 0%),
            PathElement::Quadratic(97.5%, 0%),
            PathElement::Point(97.5%, 17%),
            PathElement::Line,
            PathElement::Point(100%, 100%),
            PathElement::Close
        ]
    }
    .example_host_tab_idle {
        fill: rgb(38, 38, 38)
        elements: [
            PathElement::Point(0%, 100%),
            PathElement::Line,
            PathElement::Point(2.5%, 17%),
            PathElement::Quadratic(2.5%, 0%),
            PathElement::Point(5%, 0%),
            PathElement::Line,
            PathElement::Point(95%, 0%),
            PathElement::Quadratic(97.5%, 0%),
            PathElement::Point(97.5%, 17%),
            PathElement::Line,
            PathElement::Point(100%, 100%),
            PathElement::Close
        ]
    }
}
)]
pub struct ExampleHostSourceTab {
    // Source represented by this tab.
    pub source: Property<ExampleSource>,
    // Source index represented by this tab.
    pub index: Property<usize>,
    // Two-way bound to the owning ExampleHost's selected_source.
    pub selected: Property<usize>,
}

impl ExampleHostSourceTab {
    // Updates the parent ExampleHost's selected source.
    pub fn select_source(&mut self, _ctx: &NodeContext, _event: Event<Click>) {
        self.selected.set(self.index.get());
    }
}

fn fallback_source() -> ExampleSource {
    ExampleSource {
        label: "No source".to_string(),
        language: "text".to_string(),
        code: "No source files were provided.".to_string(),
    }
}

fn selected_source_markup(sources: &[ExampleSource], selected: usize, light: bool) -> String {
    let source = sources.get(selected.min(sources.len().saturating_sub(1)));
    match source {
        Some(source) => highlighted_code_markup(source, light),
        None => highlighted_code_markup(&fallback_source(), light),
    }
}

fn highlighted_code_markup(source: &ExampleSource, light: bool) -> String {
    let palette = SourcePalette::for_light_mode(light);
    let mut out = format!(
        "<pre data-pax-code-markup=\"example-host\" style=\"margin:0; white-space:pre; line-height:18px; color:{};\">", palette.foreground,
    );
    for line in source.code.lines() {
        highlight_line(line, source.language.as_str(), palette, &mut out);
        out.push('\n');
    }
    out.push_str("</pre>");
    out
}

fn highlight_line(line: &str, language: &str, palette: SourcePalette, out: &mut String) {
    let bytes = line.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        let rest = &line[index..];
        if rest.starts_with("//") {
            push_span(out, palette.comment, rest);
            return;
        }

        let ch = rest.chars().next().unwrap_or_default();
        if ch == '"' {
            let end = string_end(rest, '"');
            push_span(out, CODE_STRING, &rest[..end]);
            index += end;
            continue;
        }
        if ch == '\'' {
            let end = string_end(rest, '\'');
            push_span(out, CODE_STRING, &rest[..end]);
            index += end;
            continue;
        }
        if ch.is_ascii_digit() {
            let len = token_len(rest, |c| c.is_ascii_digit() || c == '.');
            push_span(out, CODE_NUMBER, &rest[..len]);
            index += len;
            continue;
        }
        if is_ident_start(ch) {
            let len = token_len(rest, is_ident_continue);
            let token = &rest[..len];
            let following = rest[len..].trim_start();
            if is_keyword(token, language) {
                push_span(out, CODE_KEYWORD, token);
            } else if token.chars().next().is_some_and(|c| c.is_ascii_uppercase()) {
                push_span(out, CODE_TYPE, token);
            } else if following.starts_with('(')
                || (following.starts_with('!') && !following.starts_with("!="))
            {
                push_span(out, CODE_FUNCTION, token);
            } else if (following.starts_with('=') && !following.starts_with("=="))
                || (following.starts_with(':') && !following.starts_with("::"))
            {
                push_span(out, CODE_PROPERTY, token);
            } else {
                escape_html_into(out, token);
            }
            index += len;
            continue;
        }
        if "{}[]()<>=+-*/.,:;!&|?%@".contains(ch) {
            push_span(out, palette.foreground, &rest[..ch.len_utf8()]);
            index += ch.len_utf8();
            continue;
        }

        escape_html_into(out, &rest[..ch.len_utf8()]);
        index += ch.len_utf8();
    }
}

fn string_end(input: &str, quote: char) -> usize {
    let mut escaped = false;
    for (offset, ch) in input.char_indices().skip(1) {
        if escaped {
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        if ch == quote {
            return offset + ch.len_utf8();
        }
    }
    input.len()
}

fn token_len(input: &str, predicate: impl Fn(char) -> bool) -> usize {
    input
        .char_indices()
        .find_map(|(offset, ch)| (!predicate(ch)).then_some(offset))
        .unwrap_or(input.len())
}

fn is_ident_start(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphabetic()
}

fn is_ident_continue(ch: char) -> bool {
    is_ident_start(ch) || ch.is_ascii_digit()
}

fn is_keyword(token: &str, language: &str) -> bool {
    matches!(
        token,
        "as" | "bind"
            | "break"
            | "const"
            | "continue"
            | "else"
            | "enum"
            | "false"
            | "fn"
            | "for"
            | "if"
            | "impl"
            | "in"
            | "let"
            | "match"
            | "mod"
            | "move"
            | "mut"
            | "pub"
            | "return"
            | "self"
            | "Self"
            | "slot"
            | "struct"
            | "true"
            | "type"
            | "use"
            | "where"
            | "while"
    ) || (language == "pax" && matches!(token, "import" | "settings"))
}

fn push_span(out: &mut String, color: &str, text: &str) {
    out.push_str("<span style=\"color:");
    out.push_str(color);
    out.push_str("\">");
    escape_html_into(out, text);
    out.push_str("</span>");
}

fn escape_html_into(out: &mut String, text: &str) {
    for ch in text.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(ch),
        }
    }
}
