#![allow(unused_imports)]

use crate::*;
use pax_engine::api::{
    cursor::CursorStyle, Click, Duration, EasingCurve, Event, MouseDown, MouseMove, MouseOut,
    MouseOver, MouseUp, Store,
};
use pax_engine::*;
use pax_runtime::api::NodeContext;

const DESKTOP_DRAWER_WIDTH_PX: f64 = 430.0;
const MIN_PREVIEW_WIDTH_PX: f64 = 320.0;
const MIN_DRAWER_WIDTH_PX: f64 = 280.0;
const RESIZE_MIN_DELTA_PX: f64 = 2.0;
const DRAWER_EASE_MS: u64 = 250;

/// A display wrapper for documentation/demo examples.
///
/// `ExampleHost` renders every projected child with `slot()` and offers an
/// optional source drawer driven by an explicit `sources` manifest.
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

    // Private selected source after clamping/fallback.
    pub _active_source: Property<ExampleSource>,
    // Private syntax-highlighted source markup for every source.
    pub _source_markups: Property<Vec<String>>,
    // Private syntax-highlighted source markup for the code pane.
    pub _active_source_markup: Property<String>,
    // Private source-panel width in pixels, remembered while the drawer closes.
    pub _drawer_width_px: Property<f64>,
    // Private animated source drawer progress, from closed 0.0 to open 1.0.
    pub _drawer_progress: Property<f64>,
    // Private last observed public drawer state, used for external control sync.
    pub _drawer_target_open: Property<bool>,
    // Private drag state for the source divider.
    pub _is_resizing_drawer: Property<bool>,
}

impl Default for ExampleHost {
    fn default() -> Self {
        Self {
            title: Property::new("Example".to_string()),
            sources: Property::new(vec![]),
            selected_source: Property::new(0),
            drawer_open: Property::new(false),
            _active_source: Property::new(fallback_source()),
            _source_markups: Property::new(vec![highlighted_code_markup(&fallback_source())]),
            _active_source_markup: Property::new(highlighted_code_markup(&fallback_source())),
            _drawer_width_px: Property::new(DESKTOP_DRAWER_WIDTH_PX),
            _drawer_progress: Property::new(0.0),
            _drawer_target_open: Property::new(false),
            _is_resizing_drawer: Property::new(false),
        }
    }
}

impl ExampleHost {
    // Wires selected-source lookup and makes child tabs update this host.
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        ctx.push_local_store(SelectedSourceStore(self.selected_source.clone()));
        let drawer_open = self.drawer_open.get();
        self._drawer_target_open.set(drawer_open);
        self._drawer_progress
            .set(if drawer_open { 1.0 } else { 0.0 });

        let sources = self.sources.clone();
        let selected_source = self.selected_source.clone();
        let deps = [sources.untyped(), selected_source.untyped()];
        self._active_source.replace_with(Property::computed(
            move || active_source(&sources.get(), selected_source.get()),
            &deps,
        ));

        let sources_for_markup = self.sources.clone();
        let deps = [sources_for_markup.untyped()];
        self._source_markups.replace_with(Property::computed(
            move || source_markups(&sources_for_markup.get()),
            &deps,
        ));

        let source_markups = self._source_markups.clone();
        let selected_source = self.selected_source.clone();
        let deps = [source_markups.untyped(), selected_source.untyped()];
        self._active_source_markup.replace_with(Property::computed(
            move || {
                let source_markups = source_markups.get();
                if source_markups.is_empty() {
                    return highlighted_code_markup(&fallback_source());
                }
                source_markups
                    .get(selected_source.get().min(source_markups.len() - 1))
                    .cloned()
                    .unwrap_or_else(|| highlighted_code_markup(&fallback_source()))
            },
            &deps,
        ));
    }

    /// Keeps the animated drawer progress synchronized with external state writes.
    pub fn on_pre_render(&mut self, _ctx: &NodeContext) {
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
        if !self.drawer_open.get() {
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
        if !self.drawer_open.get() {
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
        self._drawer_progress.ease_to(
            target,
            Duration::Milliseconds(DRAWER_EASE_MS.into()),
            EasingCurve::InQuad,
        );
    }
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

// Internal source tab used by `ExampleHost`.
#[pax]
#[engine_import_path("pax_engine")]
#[inlined(
    <Group width=100% height=100% @click=self.select_source>
        <Text class="example_host_tab_label" x=50% y=50% width={100% - 16px} height=18px text={source.label} selectable=false clip=true />
        if self.index == self.selected {
            <Path class="example_host_tab_active" width=100% height=100% />
        }
        if self.index != self.selected {
            <Path class="example_host_tab_idle" width=100% height=100% />
        }
    </Group>

    @settings {
        .example_host_tab_label {
            style: TextStyle {
                font: Font::Web("Space Grotesk", "https://fonts.googleapis.com/css2?family=Space+Grotesk:wght@400;500;600&display=swap", FontStyle::Normal, FontWeight::Medium)
                font_size: 11px
                fill: WHITE
                align_vertical: TextAlignVertical::Center
                align_horizontal: TextAlignHorizontal::Center
                align_multiline: TextAlignHorizontal::Center
                underline: false
            }
        }

        .example_host_tab_active {
            fill: rgba(61, 103, 180, 255)
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
            fill: rgba(20, 24, 32, 255)
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
    // Currently selected source index.
    pub selected: Property<usize>,
}

impl ExampleHostSourceTab {
    // Updates the parent ExampleHost's selected source.
    pub fn select_source(&mut self, ctx: &NodeContext, _event: Event<Click>) {
        let index = self.index.get();
        let _ = ctx.peek_local_store(|SelectedSourceStore(selected): &mut SelectedSourceStore| {
            selected.set(index);
        });
    }
}

struct SelectedSourceStore(Property<usize>);
impl Store for SelectedSourceStore {}

fn active_source(sources: &[ExampleSource], selected: usize) -> ExampleSource {
    if sources.is_empty() {
        return fallback_source();
    }
    sources
        .get(selected.min(sources.len() - 1))
        .cloned()
        .unwrap_or_else(fallback_source)
}

fn fallback_source() -> ExampleSource {
    ExampleSource {
        label: "No source".to_string(),
        language: "text".to_string(),
        code: "No source files were provided.".to_string(),
    }
}

fn source_markups(sources: &[ExampleSource]) -> Vec<String> {
    if sources.is_empty() {
        return vec![highlighted_code_markup(&fallback_source())];
    }
    sources.iter().map(highlighted_code_markup).collect()
}

fn highlighted_code_markup(source: &ExampleSource) -> String {
    let mut out = String::from(
        "<pre data-pax-code-markup=\"example-host\" style=\"margin:0; white-space:pre; line-height:18px; color:#e0eafa;\">",
    );
    for line in source.code.lines() {
        highlight_line(line, source.language.as_str(), &mut out);
        out.push('\n');
    }
    out.push_str("</pre>");
    out
}

fn highlight_line(line: &str, language: &str, out: &mut String) {
    let bytes = line.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        let rest = &line[index..];
        if rest.starts_with("//") {
            push_span(out, "#7f8da8", rest);
            return;
        }

        let ch = rest.chars().next().unwrap_or_default();
        if ch == '"' {
            let end = string_end(rest, '"');
            push_span(out, "#ffd166", &rest[..end]);
            index += end;
            continue;
        }
        if ch == '\'' {
            let end = string_end(rest, '\'');
            push_span(out, "#ffd166", &rest[..end]);
            index += end;
            continue;
        }
        if ch.is_ascii_digit() {
            let len = token_len(rest, |c| c.is_ascii_digit() || c == '.');
            push_span(out, "#c792ea", &rest[..len]);
            index += len;
            continue;
        }
        if is_ident_start(ch) {
            let len = token_len(rest, is_ident_continue);
            let token = &rest[..len];
            if is_keyword(token, language) {
                push_span(out, "#82aaff", token);
            } else if token.chars().next().is_some_and(|c| c.is_ascii_uppercase()) {
                push_span(out, "#7ee787", token);
            } else {
                escape_html_into(out, token);
            }
            index += len;
            continue;
        }
        if "{}[]()<>=+-*/.,:;!&|?%@".contains(ch) {
            push_span(out, "#89ddff", &rest[..ch.len_utf8()]);
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
