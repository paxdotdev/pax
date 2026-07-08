use std::collections::HashMap;

use pax_engine::api::{
    Color, Numeric, PathElement, Property, Size, Stroke, StrokeCap, StrokeJoin, UnitValue,
};
use pax_engine::*;
use pax_runtime::api::NodeContext;

/// Renders text as single-stroke vector paths suitable for handwriting effects.
///
/// `Handwriter` uses bundled SVG stroke fonts and translates text into
/// `PathElement` data for an inner `Path`. Use `draw_start` and `draw_end` to
/// animate the visible writing range.
#[pax]
#[engine_import_path("pax_engine")]
#[custom(Default)]
#[file("drawing/handwriter.pax")]
pub struct Handwriter {
    /// Text to render. Newline characters create additional baselines.
    pub text: Property<String>,
    /// Bundled stroke font used to draw `text`.
    pub font: Property<HandwriterFont>,
    /// Stroke used for the generated path.
    pub stroke: Property<Stroke>,
    /// Start position of the visible handwriting range.
    pub draw_start: Property<UnitValue>,
    /// End position of the visible handwriting range.
    pub draw_end: Property<UnitValue>,
    /// Baseline-to-baseline multiplier relative to the font's em size.
    pub line_height: Property<f64>,
    // Generated path elements consumed by the internal `Path`.
    pub _elements: Property<Vec<PathElement>>,
}

impl Default for Handwriter {
    fn default() -> Self {
        Self {
            text: Property::new(
                "The silver apples of the moon,\nThe golden apples of the sun.".into(),
            ),
            font: Property::new(HandwriterFont::EMSAllure),
            stroke: Property::new(Stroke {
                color: Property::new(Color::BLACK),
                width: Property::new(Size::Pixels(Numeric::F64(3.0))),
                cap: Property::new(StrokeCap::Round),
                join: Property::new(StrokeJoin::Round),
            }),
            draw_start: Property::new(UnitValue::Unitless(Numeric::F64(0.0))),
            draw_end: Property::new(UnitValue::Unitless(Numeric::F64(1.0))),
            line_height: Property::new(1.2),
            _elements: Property::new(Vec::new()),
        }
    }
}

/// Bundled single-stroke fonts available to `Handwriter`.
#[pax]
#[engine_import_path("pax_engine")]
pub enum HandwriterFont {
    /// Cursive, airy script.
    #[default]
    EMSAllure,
    /// Friendly rounded print hand.
    EMSDelight,
    /// Tall invitation script.
    EMSInvite,
    /// Flowing connected script.
    EMSLeague,
    /// Casual narrow script.
    EMSNeato,
    /// Rectilinear plotter hand.
    EMSOsmotron,
    /// Highly readable manuscript hand.
    EMSReadability,
    /// Technical drafting hand.
    EMSTech,
    /// Classic Hershey sans stroke font.
    HersheySans1,
    /// Classic Hershey connected script.
    HersheyScript1,
}

impl Handwriter {
    // Wires generated path data reactively from text/font settings.
    pub fn on_mount(&mut self, _ctx: &NodeContext) {
        let text = self.text.clone();
        let font = self.font.clone();
        let line_height = self.line_height.clone();
        let deps = [text.untyped(), font.untyped(), line_height.untyped()];
        self._elements.replace_with(Property::computed(
            move || render_handwriter_text(&text.get(), font.get(), line_height.get()),
            &deps,
        ));
    }
}

fn render_handwriter_text(
    text: &str,
    font: HandwriterFont,
    line_height_multiplier: f64,
) -> Vec<PathElement> {
    let Some(svg_font) = parse_svg_font(font_source(font)) else {
        return Vec::new();
    };
    let line_height = svg_font.units_per_em * line_height_multiplier.max(0.1);
    let fallback_advance = svg_font.default_advance.max(svg_font.units_per_em * 0.35);
    let mut raw = Vec::new();
    let mut pen_x = 0.0;
    let mut line_index = 0.0;

    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '\n' => {
                line_index += 1.0;
                pen_x = 0.0;
            }
            '\\' if chars.peek() == Some(&'n') => {
                chars.next();
                line_index += 1.0;
                pen_x = 0.0;
            }
            '\r' => {}
            '\t' => {
                pen_x += fallback_advance * 4.0;
            }
            _ => {
                let baseline = line_index * line_height + svg_font.ascent;
                if let Some(glyph) = svg_font.glyphs.get(&ch) {
                    push_glyph(&mut raw, glyph, pen_x, baseline);
                    pen_x += glyph.advance;
                } else {
                    pen_x += fallback_advance;
                }
            }
        }
    }

    let bounds = compute_bounds(&raw);
    if raw.is_empty() || !bounds.is_valid() {
        return Vec::new();
    }

    let width = (bounds.max_x - bounds.min_x).max(1.0);
    let height = (bounds.max_y - bounds.min_y).max(1.0);
    let mut elements = Vec::new();
    for element in raw {
        push_path_element(&mut elements, element, &bounds, width, height);
    }
    elements
}

fn push_glyph(raw: &mut Vec<RawElement>, glyph: &Glyph, pen_x: f64, baseline: f64) {
    for element in &glyph.elements {
        match *element {
            GlyphElement::Move(point) => {
                let point = transform_glyph_point(point, pen_x, baseline);
                raw.push(RawElement::Move(point));
            }
            GlyphElement::Line(point) => {
                let point = transform_glyph_point(point, pen_x, baseline);
                raw.push(RawElement::Line(point));
            }
            GlyphElement::Quadratic(control, point) => {
                let control = transform_glyph_point(control, pen_x, baseline);
                let point = transform_glyph_point(point, pen_x, baseline);
                raw.push(RawElement::Quadratic(control, point));
            }
            GlyphElement::Cubic(control_1, control_2, point) => {
                let control_1 = transform_glyph_point(control_1, pen_x, baseline);
                let control_2 = transform_glyph_point(control_2, pen_x, baseline);
                let point = transform_glyph_point(point, pen_x, baseline);
                raw.push(RawElement::Cubic(control_1, control_2, point));
            }
            GlyphElement::Close => raw.push(RawElement::Close),
        }
    }
}

fn transform_glyph_point(point: RawPoint, pen_x: f64, baseline: f64) -> RawPoint {
    RawPoint {
        x: pen_x + point.x,
        y: baseline - point.y,
    }
}

fn compute_bounds(raw: &[RawElement]) -> Bounds {
    let mut bounds = Bounds::default();
    for element in raw {
        match *element {
            RawElement::Move(point) | RawElement::Line(point) => bounds.include(point),
            RawElement::Quadratic(control, point) => {
                bounds.include(control);
                bounds.include(point);
            }
            RawElement::Cubic(control_1, control_2, point) => {
                bounds.include(control_1);
                bounds.include(control_2);
                bounds.include(point);
            }
            RawElement::Close => {}
        }
    }
    bounds
}

#[derive(Default)]
struct Bounds {
    min_x: f64,
    min_y: f64,
    max_x: f64,
    max_y: f64,
    initialized: bool,
}

impl Bounds {
    fn include(&mut self, point: RawPoint) {
        if self.initialized {
            self.min_x = self.min_x.min(point.x);
            self.min_y = self.min_y.min(point.y);
            self.max_x = self.max_x.max(point.x);
            self.max_y = self.max_y.max(point.y);
        } else {
            self.min_x = point.x;
            self.min_y = point.y;
            self.max_x = point.x;
            self.max_y = point.y;
            self.initialized = true;
        }
    }

    fn is_valid(&self) -> bool {
        self.initialized
    }
}

#[derive(Clone)]
struct SvgFont {
    units_per_em: f64,
    ascent: f64,
    default_advance: f64,
    glyphs: HashMap<char, Glyph>,
}

#[derive(Clone)]
struct Glyph {
    advance: f64,
    elements: Vec<GlyphElement>,
}

#[derive(Clone, Copy)]
enum GlyphElement {
    Move(RawPoint),
    Line(RawPoint),
    Quadratic(RawPoint, RawPoint),
    Cubic(RawPoint, RawPoint, RawPoint),
    Close,
}

#[derive(Clone, Copy)]
enum RawElement {
    Move(RawPoint),
    Line(RawPoint),
    Quadratic(RawPoint, RawPoint),
    Cubic(RawPoint, RawPoint, RawPoint),
    Close,
}

fn push_path_element(
    elements: &mut Vec<PathElement>,
    element: RawElement,
    bounds: &Bounds,
    width: f64,
    height: f64,
) {
    match element {
        RawElement::Move(point) => elements.push(path_point(point, bounds, width, height)),
        RawElement::Line(point) => {
            elements.push(PathElement::Line);
            elements.push(path_point(point, bounds, width, height));
        }
        RawElement::Quadratic(control, point) => {
            elements.push(PathElement::Quadratic(
                percent_x(control, bounds, width),
                percent_y(control, bounds, height),
            ));
            elements.push(path_point(point, bounds, width, height));
        }
        RawElement::Cubic(control_1, control_2, point) => {
            elements.push(PathElement::Cubic(
                percent_x(control_1, bounds, width),
                percent_y(control_1, bounds, height),
                percent_x(control_2, bounds, width),
                percent_y(control_2, bounds, height),
            ));
            elements.push(path_point(point, bounds, width, height));
        }
        RawElement::Close => elements.push(PathElement::Close),
    }
}

#[derive(Clone, Copy, Default)]
struct RawPoint {
    x: f64,
    y: f64,
}

fn percent_x(point: RawPoint, bounds: &Bounds, width: f64) -> Size {
    percent(((point.x - bounds.min_x) / width) * 94.0 + 3.0)
}

fn percent_y(point: RawPoint, bounds: &Bounds, height: f64) -> Size {
    percent(((point.y - bounds.min_y) / height) * 94.0 + 3.0)
}

fn path_point(point: RawPoint, bounds: &Bounds, width: f64, height: f64) -> PathElement {
    PathElement::Point(
        percent_x(point, bounds, width),
        percent_y(point, bounds, height),
    )
}

fn percent(value: f64) -> Size {
    Size::Percent(Numeric::F64(value))
}

fn parse_svg_font(svg: &'static str) -> Option<SvgFont> {
    let font_tag = find_tag(svg, "font")?;
    let default_advance = parse_attr_f64(font_tag, "horiz-adv-x").unwrap_or(1000.0);
    let font_face_tag = find_tag(svg, "font-face");
    let units_per_em = font_face_tag
        .and_then(|tag| parse_attr_f64(tag, "units-per-em"))
        .unwrap_or(1000.0);
    let ascent = font_face_tag
        .and_then(|tag| parse_attr_f64(tag, "ascent"))
        .unwrap_or(units_per_em * 0.8);

    let mut glyphs = HashMap::new();
    for glyph_tag in find_tags(svg, "glyph") {
        let Some(unicode) = attr_value(glyph_tag, "unicode") else {
            continue;
        };
        let mut chars = unicode.chars();
        let Some(ch) = chars.next() else {
            continue;
        };
        if chars.next().is_some() {
            continue;
        }
        let advance = parse_attr_f64(glyph_tag, "horiz-adv-x").unwrap_or(default_advance);
        let elements = attr_value(glyph_tag, "d")
            .map(|data| parse_path_data(&data))
            .unwrap_or_default();
        glyphs.insert(ch, Glyph { advance, elements });
    }

    Some(SvgFont {
        units_per_em,
        ascent,
        default_advance,
        glyphs,
    })
}

fn parse_path_data(data: &str) -> Vec<GlyphElement> {
    let tokens = tokenize_path_data(data);
    let mut parser = PathDataParser {
        tokens: &tokens,
        index: 0,
        command: None,
        current: RawPoint::default(),
        contour_start: RawPoint::default(),
        last_cubic_control: None,
        last_quadratic_control: None,
        elements: Vec::new(),
    };
    parser.parse();
    parser.elements
}

struct PathDataParser<'a> {
    tokens: &'a [PathToken],
    index: usize,
    command: Option<char>,
    current: RawPoint,
    contour_start: RawPoint,
    last_cubic_control: Option<RawPoint>,
    last_quadratic_control: Option<RawPoint>,
    elements: Vec<GlyphElement>,
}

impl PathDataParser<'_> {
    fn parse(&mut self) {
        while self.index < self.tokens.len() {
            if let Some(command) = self.take_command() {
                self.command = Some(command);
            }

            let Some(command) = self.command else {
                break;
            };
            let relative = command.is_ascii_lowercase();
            match command.to_ascii_uppercase() {
                'M' => self.parse_move(relative),
                'L' => self.parse_line(relative),
                'H' => self.parse_horizontal(relative),
                'V' => self.parse_vertical(relative),
                'C' => self.parse_cubic(relative),
                'S' => self.parse_smooth_cubic(relative),
                'Q' => self.parse_quadratic(relative),
                'T' => self.parse_smooth_quadratic(relative),
                'Z' => self.close_contour(),
                _ => break,
            }
        }
    }

    fn parse_move(&mut self, relative: bool) {
        let Some(point) = self.take_point(relative) else {
            return;
        };
        self.current = point;
        self.contour_start = point;
        self.elements.push(GlyphElement::Move(point));
        self.clear_controls();

        while let Some(point) = self.take_point(relative) {
            self.current = point;
            self.elements.push(GlyphElement::Line(point));
        }
    }

    fn parse_line(&mut self, relative: bool) {
        while let Some(point) = self.take_point(relative) {
            self.current = point;
            self.elements.push(GlyphElement::Line(point));
            self.clear_controls();
        }
    }

    fn parse_horizontal(&mut self, relative: bool) {
        while let Some(x) = self.take_number() {
            let point = RawPoint {
                x: if relative { self.current.x + x } else { x },
                y: self.current.y,
            };
            self.current = point;
            self.elements.push(GlyphElement::Line(point));
            self.clear_controls();
        }
    }

    fn parse_vertical(&mut self, relative: bool) {
        while let Some(y) = self.take_number() {
            let point = RawPoint {
                x: self.current.x,
                y: if relative { self.current.y + y } else { y },
            };
            self.current = point;
            self.elements.push(GlyphElement::Line(point));
            self.clear_controls();
        }
    }

    fn parse_cubic(&mut self, relative: bool) {
        while let (Some(control_1), Some(control_2), Some(point)) = (
            self.take_point(relative),
            self.take_point(relative),
            self.take_point(relative),
        ) {
            self.current = point;
            self.last_cubic_control = Some(control_2);
            self.last_quadratic_control = None;
            self.elements
                .push(GlyphElement::Cubic(control_1, control_2, point));
        }
    }

    fn parse_smooth_cubic(&mut self, relative: bool) {
        while let (Some(control_2), Some(point)) =
            (self.take_point(relative), self.take_point(relative))
        {
            let control_1 = self
                .last_cubic_control
                .map(|control| reflect(control, self.current))
                .unwrap_or(self.current);
            self.current = point;
            self.last_cubic_control = Some(control_2);
            self.last_quadratic_control = None;
            self.elements
                .push(GlyphElement::Cubic(control_1, control_2, point));
        }
    }

    fn parse_quadratic(&mut self, relative: bool) {
        while let (Some(control), Some(point)) =
            (self.take_point(relative), self.take_point(relative))
        {
            self.current = point;
            self.last_quadratic_control = Some(control);
            self.last_cubic_control = None;
            self.elements.push(GlyphElement::Quadratic(control, point));
        }
    }

    fn parse_smooth_quadratic(&mut self, relative: bool) {
        while let Some(point) = self.take_point(relative) {
            let control = self
                .last_quadratic_control
                .map(|control| reflect(control, self.current))
                .unwrap_or(self.current);
            self.current = point;
            self.last_quadratic_control = Some(control);
            self.last_cubic_control = None;
            self.elements.push(GlyphElement::Quadratic(control, point));
        }
    }

    fn close_contour(&mut self) {
        self.elements.push(GlyphElement::Close);
        self.current = self.contour_start;
        self.clear_controls();
        self.command = None;
    }

    fn take_command(&mut self) -> Option<char> {
        match self.tokens.get(self.index) {
            Some(PathToken::Command(command)) => {
                self.index += 1;
                Some(*command)
            }
            _ => None,
        }
    }

    fn take_point(&mut self, relative: bool) -> Option<RawPoint> {
        let index = self.index;
        let Some(x) = self.take_number() else {
            return None;
        };
        let Some(y) = self.take_number() else {
            self.index = index;
            return None;
        };
        Some(if relative {
            RawPoint {
                x: self.current.x + x,
                y: self.current.y + y,
            }
        } else {
            RawPoint { x, y }
        })
    }

    fn take_number(&mut self) -> Option<f64> {
        match self.tokens.get(self.index) {
            Some(PathToken::Number(value)) => {
                self.index += 1;
                Some(*value)
            }
            _ => None,
        }
    }

    fn clear_controls(&mut self) {
        self.last_cubic_control = None;
        self.last_quadratic_control = None;
    }
}

fn reflect(point: RawPoint, around: RawPoint) -> RawPoint {
    RawPoint {
        x: around.x * 2.0 - point.x,
        y: around.y * 2.0 - point.y,
    }
}

#[derive(Clone, Copy)]
enum PathToken {
    Command(char),
    Number(f64),
}

fn tokenize_path_data(data: &str) -> Vec<PathToken> {
    let chars: Vec<char> = data.chars().collect();
    let mut tokens = Vec::new();
    let mut index = 0;
    while index < chars.len() {
        let ch = chars[index];
        if ch.is_ascii_whitespace() || ch == ',' {
            index += 1;
        } else if is_path_command(ch) {
            tokens.push(PathToken::Command(ch));
            index += 1;
        } else if is_number_start(ch) {
            let start = index;
            index += 1;
            while index < chars.len() {
                let next = chars[index];
                if next.is_ascii_digit() || next == '.' {
                    index += 1;
                } else if next == 'e' || next == 'E' {
                    index += 1;
                    if matches!(chars.get(index), Some('+') | Some('-')) {
                        index += 1;
                    }
                } else {
                    break;
                }
            }
            let value = chars[start..index]
                .iter()
                .collect::<String>()
                .parse::<f64>();
            if let Ok(value) = value {
                tokens.push(PathToken::Number(value));
            }
        } else {
            index += 1;
        }
    }
    tokens
}

fn is_path_command(ch: char) -> bool {
    matches!(
        ch,
        'M' | 'm'
            | 'L'
            | 'l'
            | 'H'
            | 'h'
            | 'V'
            | 'v'
            | 'C'
            | 'c'
            | 'S'
            | 's'
            | 'Q'
            | 'q'
            | 'T'
            | 't'
            | 'Z'
            | 'z'
    )
}

fn is_number_start(ch: char) -> bool {
    ch.is_ascii_digit() || ch == '-' || ch == '+' || ch == '.'
}

fn parse_attr_f64(tag: &str, name: &str) -> Option<f64> {
    attr_value(tag, name)?.parse().ok()
}

fn find_tag<'a>(source: &'a str, tag_name: &str) -> Option<&'a str> {
    find_tags(source, tag_name).into_iter().next()
}

fn find_tags<'a>(source: &'a str, tag_name: &str) -> Vec<&'a str> {
    let mut tags = Vec::new();
    let mut search_start = 0;
    let needle = format!("<{tag_name}");
    while let Some(relative_start) = source[search_start..].find(&needle) {
        let start = search_start + relative_start;
        let after_name = start + needle.len();
        let Some(next) = source.as_bytes().get(after_name) else {
            break;
        };
        if !next.is_ascii_whitespace() && *next != b'>' && *next != b'/' {
            search_start = after_name;
            continue;
        }
        let Some(relative_end) = source[start..].find('>') else {
            break;
        };
        let end = start + relative_end + 1;
        tags.push(&source[start..end]);
        search_start = end;
    }
    tags
}

fn attr_value(tag: &str, name: &str) -> Option<String> {
    let bytes = tag.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        while index < bytes.len() && !is_attr_name_start(bytes[index]) {
            index += 1;
        }
        let name_start = index;
        while index < bytes.len() && is_attr_name_char(bytes[index]) {
            index += 1;
        }
        if name_start == index {
            break;
        }
        let key = &tag[name_start..index];
        while index < bytes.len() && bytes[index].is_ascii_whitespace() {
            index += 1;
        }
        if bytes.get(index) != Some(&b'=') {
            continue;
        }
        index += 1;
        while index < bytes.len() && bytes[index].is_ascii_whitespace() {
            index += 1;
        }
        let Some(quote) = bytes.get(index).copied() else {
            break;
        };
        if quote != b'"' && quote != b'\'' {
            index += 1;
            continue;
        }
        index += 1;
        let value_start = index;
        while index < bytes.len() && bytes[index] != quote {
            index += 1;
        }
        let value = &tag[value_start..index];
        if key == name {
            return Some(decode_xml_entities(value));
        }
        index += 1;
    }
    None
}

fn is_attr_name_start(byte: u8) -> bool {
    byte.is_ascii_alphabetic() || byte == b'_'
}

fn is_attr_name_char(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b':')
}

fn decode_xml_entities(value: &str) -> String {
    let mut output = String::new();
    let mut rest = value;
    while let Some(start) = rest.find('&') {
        output.push_str(&rest[..start]);
        let entity_rest = &rest[start + 1..];
        let Some(end) = entity_rest.find(';') else {
            output.push('&');
            rest = entity_rest;
            continue;
        };
        let entity = &entity_rest[..end];
        match decode_xml_entity(entity) {
            Some(ch) => output.push(ch),
            None => {
                output.push('&');
                output.push_str(entity);
                output.push(';');
            }
        }
        rest = &entity_rest[end + 1..];
    }
    output.push_str(rest);
    output
}

fn decode_xml_entity(entity: &str) -> Option<char> {
    match entity {
        "amp" => Some('&'),
        "apos" => Some('\''),
        "gt" => Some('>'),
        "lt" => Some('<'),
        "quot" => Some('"'),
        _ if entity.starts_with("#x") => u32::from_str_radix(&entity[2..], 16)
            .ok()
            .and_then(char::from_u32),
        _ if entity.starts_with('#') => entity[1..].parse::<u32>().ok().and_then(char::from_u32),
        _ => None,
    }
}

fn font_source(font: HandwriterFont) -> &'static str {
    match font {
        HandwriterFont::EMSAllure => include_str!("handwriter_fonts/EMSAllure.svg"),
        HandwriterFont::EMSDelight => include_str!("handwriter_fonts/EMSDelight.svg"),
        HandwriterFont::EMSInvite => include_str!("handwriter_fonts/EMSInvite.svg"),
        HandwriterFont::EMSLeague => include_str!("handwriter_fonts/EMSLeague.svg"),
        HandwriterFont::EMSNeato => include_str!("handwriter_fonts/EMSNeato.svg"),
        HandwriterFont::EMSOsmotron => include_str!("handwriter_fonts/EMSOsmotron.svg"),
        HandwriterFont::EMSReadability => include_str!("handwriter_fonts/EMSReadability.svg"),
        HandwriterFont::EMSTech => include_str!("handwriter_fonts/EMSTech.svg"),
        HandwriterFont::HersheySans1 => include_str!("handwriter_fonts/HersheySans1.svg"),
        HandwriterFont::HersheyScript1 => include_str!("handwriter_fonts/HersheyScript1.svg"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_selected_font_glyphs() {
        let elements = render_handwriter_text("Yeats", HandwriterFont::HersheyScript1, 1.2);
        assert!(elements.len() > 10);
        assert!(matches!(elements.first(), Some(PathElement::Point(_, _))));
    }

    #[test]
    fn attr_parser_does_not_confuse_id_with_d() {
        let tag = r#"<font id="HersheyScript1" horiz-adv-x="378">"#;
        assert_eq!(attr_value(tag, "id").as_deref(), Some("HersheyScript1"));
        assert_eq!(attr_value(tag, "d"), None);
    }

    #[test]
    fn decodes_numeric_xml_entities() {
        assert_eq!(decode_xml_entities("&#x22;&amp;&#62;"), "\"&>");
    }
}
