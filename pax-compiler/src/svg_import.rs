use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::Path;

use color_eyre::eyre::{eyre, Report, Result, WrapErr};
use roxmltree::Document;
use sha2::{Digest, Sha256};
use svgtypes::{NumberListParser, PathParser, PathSegment, Transform as SvgTransform};

#[derive(Debug)]
pub struct SvgImport {
    pub pax_source: String,
    pub source_sha256: String,
    pub view_box: ViewBox,
    pub svg_path_count: usize,
    pub svg_polygon_count: usize,
    pub generated_path_count: usize,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Copy)]
pub struct ViewBox {
    pub min_x: f64,
    pub min_y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Color {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Paint {
    None,
    Color(Color),
}

#[derive(Debug, Clone, PartialEq)]
struct StrokeStyle {
    color: Color,
    width_px: f64,
    cap: Option<String>,
    join: Option<String>,
}

#[derive(Debug, Clone)]
struct SvgPath {
    elements: Vec<String>,
    fill: Paint,
    stroke: Option<StrokeStyle>,
}

#[derive(Debug, Clone)]
struct PaxPathNode {
    elements: Vec<String>,
    fill: Paint,
    stroke: Option<StrokeStyle>,
    bind_draw_range: bool,
}

pub fn import_svg_file(source_path: &Path) -> Result<SvgImport, Report> {
    let source = fs::read_to_string(source_path)
        .wrap_err_with(|| format!("failed to read SVG {}", source_path.display()))?;
    let sha256 = sha256_hex(source.as_bytes());
    import_svg(&source, source_path, &sha256)
}

pub fn import_svg(
    svg_source: &str,
    source_path: &Path,
    source_sha256: &str,
) -> Result<SvgImport, Report> {
    let doc = Document::parse(svg_source).wrap_err("failed to parse SVG XML")?;
    let svg = doc
        .descendants()
        .find(|node| node.is_element() && node.tag_name().name() == "svg")
        .ok_or_else(|| eyre!("missing <svg> root"))?;
    let view_box = parse_view_box(
        svg.attribute("viewBox")
            .ok_or_else(|| eyre!("SVG import requires a valid viewBox"))?,
    )?;
    let class_styles = collect_class_styles(&doc);
    let mut warnings = Vec::new();
    warn_unsupported_elements(&doc, &mut warnings);

    let mut paths = Vec::new();
    let mut svg_path_count = 0;
    let mut svg_polygon_count = 0;
    for node in doc
        .descendants()
        .filter(|node| node.is_element() && matches!(node.tag_name().name(), "path" | "polygon"))
    {
        let transform = computed_transform(&node)?;
        let mut style = computed_path_style(&node, &class_styles)?;
        if let Some(stroke) = style.stroke.as_mut() {
            stroke.width_px *= stroke_scale(transform);
        }
        let elements = match node.tag_name().name() {
            "path" => {
                svg_path_count += 1;
                let d = node
                    .attribute("d")
                    .ok_or_else(|| eyre!("<path> is missing required d attribute"))?;
                parse_path_elements(d, view_box, transform)?
            }
            "polygon" => {
                svg_polygon_count += 1;
                let points = node
                    .attribute("points")
                    .ok_or_else(|| eyre!("<polygon> is missing required points attribute"))?;
                parse_polygon_elements(points, view_box, transform)?
            }
            _ => unreachable!(),
        };
        paths.push(SvgPath {
            elements,
            fill: style.fill,
            stroke: style.stroke,
        });
    }

    if paths.is_empty() {
        return Err(eyre!(
            "SVG import found no supported <path> or <polygon> elements"
        ));
    }

    let nodes = merge_compatible_stroke_paths(paths);
    let pax_source = render_pax_template(source_path, source_sha256, view_box, &nodes);
    Ok(SvgImport {
        pax_source,
        source_sha256: source_sha256.to_string(),
        view_box,
        svg_path_count,
        svg_polygon_count,
        generated_path_count: nodes.len(),
        warnings,
    })
}

fn parse_view_box(value: &str) -> Result<ViewBox, Report> {
    let parts: Vec<f64> = value
        .replace(',', " ")
        .split_whitespace()
        .map(str::parse::<f64>)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| eyre!("invalid viewBox `{value}`"))?;
    if parts.len() != 4 || parts[2] <= 0.0 || parts[3] <= 0.0 {
        return Err(eyre!("invalid viewBox `{value}`"));
    }
    Ok(ViewBox {
        min_x: parts[0],
        min_y: parts[1],
        width: parts[2],
        height: parts[3],
    })
}

fn warn_unsupported_elements(doc: &Document<'_>, warnings: &mut Vec<String>) {
    let supported = [
        "svg", "path", "polygon", "defs", "style", "title", "desc", "metadata", "g",
    ];
    let mut names = BTreeMap::new();
    for node in doc.descendants().filter(|node| node.is_element()) {
        let name = node.tag_name().name();
        if !supported.contains(&name) {
            *names.entry(name.to_string()).or_insert(0usize) += 1;
        }
    }
    for (name, count) in names {
        warnings.push(format!(
            "ignored unsupported <{name}> element{}",
            if count == 1 {
                String::new()
            } else {
                format!(" x{count}")
            }
        ));
    }
}

fn collect_class_styles(doc: &Document<'_>) -> HashMap<String, HashMap<String, String>> {
    let mut result = HashMap::new();
    for node in doc
        .descendants()
        .filter(|node| node.is_element() && node.tag_name().name() == "style")
    {
        let Some(text) = node.text() else {
            continue;
        };
        for block in text.split('}') {
            let Some((selector, body)) = block.split_once('{') else {
                continue;
            };
            for selector in selector.split(',') {
                let selector = selector.trim();
                if !selector.starts_with('.') {
                    continue;
                }
                let class_name = selector
                    .trim_start_matches('.')
                    .split_whitespace()
                    .next()
                    .unwrap_or_default();
                if class_name.is_empty() {
                    continue;
                }
                result.insert(class_name.to_string(), parse_style_declarations(body));
            }
        }
    }
    result
}

fn parse_style_declarations(style: &str) -> HashMap<String, String> {
    style
        .split(';')
        .filter_map(|declaration| {
            let (key, value) = declaration.split_once(':')?;
            Some((key.trim().to_string(), value.trim().to_string()))
        })
        .collect()
}

struct ComputedStyle {
    fill: Paint,
    stroke: Option<StrokeStyle>,
}

fn computed_path_style(
    node: &roxmltree::Node<'_, '_>,
    class_styles: &HashMap<String, HashMap<String, String>>,
) -> Result<ComputedStyle, Report> {
    let mut values = HashMap::new();
    let mut nodes: Vec<_> = node.ancestors().filter(|node| node.is_element()).collect();
    nodes.reverse();
    for node in nodes {
        apply_style_values(&node, class_styles, &mut values);
    }

    let opacity = parse_alpha(values.get("opacity").map(String::as_str)).unwrap_or(1.0);
    let fill_opacity =
        opacity * parse_alpha(values.get("fill-opacity").map(String::as_str)).unwrap_or(1.0);
    let stroke_opacity =
        opacity * parse_alpha(values.get("stroke-opacity").map(String::as_str)).unwrap_or(1.0);
    let fill_value = values.get("fill").map(String::as_str).unwrap_or("black");
    let fill = parse_paint(fill_value, fill_opacity)
        .wrap_err_with(|| format!("unsupported fill paint `{fill_value}`"))?;
    let stroke_value = values.get("stroke").map(String::as_str).unwrap_or("none");
    let stroke_paint = parse_paint(stroke_value, stroke_opacity)
        .wrap_err_with(|| format!("unsupported stroke paint `{stroke_value}`"))?;
    let stroke = match stroke_paint {
        Paint::Color(color) => Some(StrokeStyle {
            color,
            width_px: parse_length_px(values.get("stroke-width").map(String::as_str))
                .unwrap_or(1.0),
            cap: parse_stroke_cap(values.get("stroke-linecap").map(String::as_str)),
            join: parse_stroke_join(values.get("stroke-linejoin").map(String::as_str)),
        }),
        Paint::None => None,
    };

    Ok(ComputedStyle { fill, stroke })
}

fn apply_style_values(
    node: &roxmltree::Node<'_, '_>,
    class_styles: &HashMap<String, HashMap<String, String>>,
    values: &mut HashMap<String, String>,
) {
    if let Some(classes) = node.attribute("class") {
        for class_name in classes.split_whitespace() {
            if let Some(style) = class_styles.get(class_name) {
                for (key, value) in style {
                    values.insert(key.to_string(), value.to_string());
                }
            }
        }
    }
    for attr in [
        "fill",
        "fill-opacity",
        "opacity",
        "stroke",
        "stroke-opacity",
        "stroke-width",
        "stroke-linecap",
        "stroke-linejoin",
    ] {
        if let Some(value) = node.attribute(attr) {
            values.insert(attr.to_string(), value.to_string());
        }
    }
    if let Some(style) = node.attribute("style") {
        for (key, value) in parse_style_declarations(style) {
            values.insert(key, value);
        }
    }
}

fn parse_alpha(value: Option<&str>) -> Option<f64> {
    let value = value?.trim();
    if let Some(percent) = value.strip_suffix('%') {
        percent.trim().parse::<f64>().ok().map(|v| v / 100.0)
    } else {
        value.parse::<f64>().ok()
    }
    .map(|value| value.clamp(0.0, 1.0))
}

fn parse_length_px(value: Option<&str>) -> Option<f64> {
    let value = value?.trim();
    let value = value.strip_suffix("px").unwrap_or(value);
    value.parse::<f64>().ok()
}

fn parse_stroke_cap(value: Option<&str>) -> Option<String> {
    match value.map(str::trim) {
        Some("round") => Some("StrokeCap::Round".to_string()),
        Some("square") => Some("StrokeCap::Square".to_string()),
        _ => None,
    }
}

fn parse_stroke_join(value: Option<&str>) -> Option<String> {
    match value.map(str::trim) {
        Some("round") => Some("StrokeJoin::Round".to_string()),
        Some("bevel") => Some("StrokeJoin::Bevel".to_string()),
        _ => None,
    }
}

fn parse_paint(value: &str, opacity: f64) -> Result<Paint, Report> {
    let value = value.trim();
    if value.eq_ignore_ascii_case("none") {
        return Ok(Paint::None);
    }
    parse_color(value, opacity).map(Paint::Color)
}

fn parse_color(value: &str, opacity: f64) -> Result<Color, Report> {
    let value = value.trim();
    let (r, g, b, base_alpha) = if let Some(hex) = value.strip_prefix('#') {
        match hex.len() {
            3 => {
                let r = u8::from_str_radix(&hex[0..1].repeat(2), 16)?;
                let g = u8::from_str_radix(&hex[1..2].repeat(2), 16)?;
                let b = u8::from_str_radix(&hex[2..3].repeat(2), 16)?;
                (r, g, b, 255)
            }
            6 => {
                let r = u8::from_str_radix(&hex[0..2], 16)?;
                let g = u8::from_str_radix(&hex[2..4], 16)?;
                let b = u8::from_str_radix(&hex[4..6], 16)?;
                (r, g, b, 255)
            }
            8 => {
                let r = u8::from_str_radix(&hex[0..2], 16)?;
                let g = u8::from_str_radix(&hex[2..4], 16)?;
                let b = u8::from_str_radix(&hex[4..6], 16)?;
                let a = u8::from_str_radix(&hex[6..8], 16)?;
                (r, g, b, a)
            }
            _ => return Err(eyre!("unsupported color `{value}`")),
        }
    } else {
        match value.to_ascii_lowercase().as_str() {
            "black" => (0, 0, 0, 255),
            "white" => (255, 255, 255, 255),
            "transparent" => (0, 0, 0, 0),
            _ => return Err(eyre!("unsupported color `{value}`")),
        }
    };
    Ok(Color {
        r,
        g,
        b,
        a: ((base_alpha as f64) * opacity).round().clamp(0.0, 255.0) as u8,
    })
}

fn computed_transform(node: &roxmltree::Node<'_, '_>) -> Result<SvgTransform, Report> {
    let mut transform = identity_transform();
    let mut nodes: Vec<_> = node.ancestors().filter(|node| node.is_element()).collect();
    nodes.reverse();
    for node in nodes {
        if let Some(value) = node.attribute("transform") {
            if !value.trim().is_empty() {
                let parsed = value.parse::<SvgTransform>().map_err(|error| {
                    eyre!(
                        "unsupported transform on <{}>: `{}` ({error:?})",
                        node.tag_name().name(),
                        value
                    )
                })?;
                transform = multiply_transforms(transform, parsed);
            }
        }
    }
    Ok(transform)
}

fn identity_transform() -> SvgTransform {
    SvgTransform::new(1.0, 0.0, 0.0, 1.0, 0.0, 0.0)
}

fn multiply_transforms(left: SvgTransform, right: SvgTransform) -> SvgTransform {
    SvgTransform::new(
        left.a * right.a + left.c * right.b,
        left.b * right.a + left.d * right.b,
        left.a * right.c + left.c * right.d,
        left.b * right.c + left.d * right.d,
        left.a * right.e + left.c * right.f + left.e,
        left.b * right.e + left.d * right.f + left.f,
    )
}

fn stroke_scale(transform: SvgTransform) -> f64 {
    let x_scale = transform.a.hypot(transform.b);
    let y_scale = transform.c.hypot(transform.d);
    ((x_scale + y_scale) / 2.0).max(0.0001)
}

fn apply_transform(point: Point, transform: SvgTransform) -> Point {
    Point {
        x: transform.a * point.x + transform.c * point.y + transform.e,
        y: transform.b * point.x + transform.d * point.y + transform.f,
    }
}

fn parse_path_elements(
    data: &str,
    view_box: ViewBox,
    transform: SvgTransform,
) -> Result<Vec<String>, Report> {
    let mut elements = Vec::new();
    let mut current = Point { x: 0.0, y: 0.0 };
    let mut subpath_start = current;
    let mut previous_cubic_control: Option<Point> = None;
    let mut previous_quadratic_control: Option<Point> = None;

    for segment in PathParser::from(data) {
        let segment = segment.map_err(|error| eyre!("malformed SVG path data: {error:?}"))?;
        match segment {
            PathSegment::MoveTo { abs, x, y } => {
                current = resolve_point(abs, x, y, current);
                subpath_start = current;
                elements.push(point_element(current, view_box, transform));
                previous_cubic_control = None;
                previous_quadratic_control = None;
            }
            PathSegment::LineTo { abs, x, y } => {
                let next = resolve_point(abs, x, y, current);
                push_line_if_visible(&mut elements, current, next, view_box, transform);
                current = next;
                previous_cubic_control = None;
                previous_quadratic_control = None;
            }
            PathSegment::HorizontalLineTo { abs, x } => {
                let next = Point {
                    x: if abs { x } else { current.x + x },
                    y: current.y,
                };
                push_line_if_visible(&mut elements, current, next, view_box, transform);
                current = next;
                previous_cubic_control = None;
                previous_quadratic_control = None;
            }
            PathSegment::VerticalLineTo { abs, y } => {
                let next = Point {
                    x: current.x,
                    y: if abs { y } else { current.y + y },
                };
                push_line_if_visible(&mut elements, current, next, view_box, transform);
                current = next;
                previous_cubic_control = None;
                previous_quadratic_control = None;
            }
            PathSegment::CurveTo {
                abs,
                x1,
                y1,
                x2,
                y2,
                x,
                y,
            } => {
                let control_1 = resolve_point(abs, x1, y1, current);
                let control_2 = resolve_point(abs, x2, y2, current);
                current = resolve_point(abs, x, y, current);
                elements.push(cubic_element(control_1, control_2, view_box, transform));
                elements.push(point_element(current, view_box, transform));
                previous_cubic_control = Some(control_2);
                previous_quadratic_control = None;
            }
            PathSegment::SmoothCurveTo { abs, x2, y2, x, y } => {
                let control_1 = previous_cubic_control
                    .map(|control| reflect(control, current))
                    .unwrap_or(current);
                let control_2 = resolve_point(abs, x2, y2, current);
                current = resolve_point(abs, x, y, current);
                elements.push(cubic_element(control_1, control_2, view_box, transform));
                elements.push(point_element(current, view_box, transform));
                previous_cubic_control = Some(control_2);
                previous_quadratic_control = None;
            }
            PathSegment::Quadratic { abs, x1, y1, x, y } => {
                let control = resolve_point(abs, x1, y1, current);
                current = resolve_point(abs, x, y, current);
                elements.push(quadratic_element(control, view_box, transform));
                elements.push(point_element(current, view_box, transform));
                previous_cubic_control = None;
                previous_quadratic_control = Some(control);
            }
            PathSegment::SmoothQuadratic { abs, x, y } => {
                let control = previous_quadratic_control
                    .map(|control| reflect(control, current))
                    .unwrap_or(current);
                current = resolve_point(abs, x, y, current);
                elements.push(quadratic_element(control, view_box, transform));
                elements.push(point_element(current, view_box, transform));
                previous_cubic_control = None;
                previous_quadratic_control = Some(control);
            }
            PathSegment::EllipticalArc { .. } => {
                return Err(eyre!(
                    "unsupported SVG path command A/a; convert arcs to cubic curves before import"
                ));
            }
            PathSegment::ClosePath { .. } => {
                elements.push("PathElement::Close".to_string());
                current = subpath_start;
                previous_cubic_control = None;
                previous_quadratic_control = None;
            }
        }
    }

    Ok(elements)
}

fn parse_polygon_elements(
    points: &str,
    view_box: ViewBox,
    transform: SvgTransform,
) -> Result<Vec<String>, Report> {
    let coordinates = NumberListParser::from(points)
        .map(|value| value.map_err(|error| eyre!("malformed SVG polygon points: {error:?}")))
        .collect::<Result<Vec<_>, _>>()?;

    if coordinates.len() % 2 != 0 {
        return Err(eyre!(
            "malformed SVG polygon points: expected coordinate pairs, found {} values",
            coordinates.len()
        ));
    }

    let points = coordinates
        .chunks_exact(2)
        .map(|pair| Point {
            x: pair[0],
            y: pair[1],
        })
        .collect::<Vec<_>>();
    if points.len() < 3 {
        return Err(eyre!(
            "SVG <polygon> requires at least three coordinate pairs"
        ));
    }

    let mut canonical_points = Vec::with_capacity(points.len());
    for point in points {
        let rendered = point_element(point, view_box, transform);
        if canonical_points
            .last()
            .is_some_and(|(_, previous): &(Point, String)| previous == &rendered)
        {
            continue;
        }
        canonical_points.push((point, rendered));
    }
    if canonical_points.len() > 1
        && canonical_points.first().map(|(_, point)| point)
            == canonical_points.last().map(|(_, point)| point)
    {
        canonical_points.pop();
    }
    if canonical_points.len() < 3 {
        return Err(eyre!(
            "SVG <polygon> requires at least three distinct rendered vertices"
        ));
    }

    let mut elements = Vec::with_capacity(canonical_points.len() * 2);
    elements.push(canonical_points[0].1.clone());
    for (_, point) in canonical_points.iter().skip(1) {
        elements.push("PathElement::Line".to_string());
        elements.push(point.clone());
    }
    elements.push("PathElement::Close".to_string());
    Ok(elements)
}

#[derive(Clone, Copy, Debug)]
struct Point {
    x: f64,
    y: f64,
}

fn resolve_point(abs: bool, x: f64, y: f64, current: Point) -> Point {
    if abs {
        Point { x, y }
    } else {
        Point {
            x: current.x + x,
            y: current.y + y,
        }
    }
}

fn reflect(control: Point, around: Point) -> Point {
    Point {
        x: (2.0 * around.x) - control.x,
        y: (2.0 * around.y) - control.y,
    }
}

fn push_line_if_visible(
    elements: &mut Vec<String>,
    from: Point,
    to: Point,
    view_box: ViewBox,
    transform: SvgTransform,
) {
    let from = point_element(from, view_box, transform);
    let to = point_element(to, view_box, transform);
    if from == to {
        return;
    }
    elements.push("PathElement::Line".to_string());
    elements.push(to);
}

fn point_element(point: Point, view_box: ViewBox, transform: SvgTransform) -> String {
    let point = apply_transform(point, transform);
    format!(
        "PathElement::Point({}, {})",
        format_percent((point.x - view_box.min_x) / view_box.width * 100.0),
        format_percent((point.y - view_box.min_y) / view_box.height * 100.0)
    )
}

fn quadratic_element(control: Point, view_box: ViewBox, transform: SvgTransform) -> String {
    let control = apply_transform(control, transform);
    format!(
        "PathElement::Quadratic({}, {})",
        format_percent((control.x - view_box.min_x) / view_box.width * 100.0),
        format_percent((control.y - view_box.min_y) / view_box.height * 100.0)
    )
}

fn cubic_element(
    control_1: Point,
    control_2: Point,
    view_box: ViewBox,
    transform: SvgTransform,
) -> String {
    let control_1 = apply_transform(control_1, transform);
    let control_2 = apply_transform(control_2, transform);
    format!(
        "PathElement::Cubic({}, {}, {}, {})",
        format_percent((control_1.x - view_box.min_x) / view_box.width * 100.0),
        format_percent((control_1.y - view_box.min_y) / view_box.height * 100.0),
        format_percent((control_2.x - view_box.min_x) / view_box.width * 100.0),
        format_percent((control_2.y - view_box.min_y) / view_box.height * 100.0)
    )
}

fn merge_compatible_stroke_paths(paths: Vec<SvgPath>) -> Vec<PaxPathNode> {
    let mut nodes = Vec::new();
    let mut pending_stroke: Option<PaxPathNode> = None;

    for path in paths {
        if path.fill == Paint::None {
            if let Some(stroke) = &path.stroke {
                if let Some(pending) = pending_stroke.as_mut() {
                    if pending.stroke.as_ref() == Some(stroke) {
                        pending.elements.extend(path.elements);
                        continue;
                    }
                }
                if let Some(pending) = pending_stroke.take() {
                    nodes.push(pending);
                }
                pending_stroke = Some(PaxPathNode {
                    elements: path.elements,
                    fill: Paint::None,
                    stroke: path.stroke,
                    bind_draw_range: true,
                });
                continue;
            }
        }
        if let Some(pending) = pending_stroke.take() {
            nodes.push(pending);
        }
        nodes.push(PaxPathNode {
            elements: path.elements,
            fill: path.fill,
            bind_draw_range: path.stroke.is_some(),
            stroke: path.stroke,
        });
    }

    if let Some(pending) = pending_stroke {
        nodes.push(pending);
    }
    nodes
}

fn render_pax_template(
    source_path: &Path,
    source_sha256: &str,
    _view_box: ViewBox,
    nodes: &[PaxPathNode],
) -> String {
    let mut out = String::new();
    out.push_str("// @generated by pax-cli svg-import\n");
    out.push_str(&format!("// source: {}\n", source_path.display()));
    out.push_str(&format!("// source_sha256: {source_sha256}\n"));
    out.push_str("<Group width=100% height=100%>\n");
    for node in nodes.iter().rev() {
        out.push_str(&render_path_node(node));
    }
    out.push_str("</Group>\n");
    out
}

fn render_path_node(node: &PaxPathNode) -> String {
    let mut out = String::new();
    out.push_str("    <Path\n");
    out.push_str("        width=100%\n");
    out.push_str("        height=100%\n");
    out.push_str(&format!("        fill={}\n", format_fill(&node.fill)));
    if let Some(stroke) = &node.stroke {
        out.push_str(&format!("        stroke={{ {} }}\n", format_stroke(stroke)));
    }
    if node.bind_draw_range {
        out.push_str("        draw_start={draw_start}\n");
        out.push_str("        draw_end={draw_end}\n");
    }
    out.push_str(&format!(
        "        elements=[{}]\n",
        node.elements.join(", ")
    ));
    out.push_str("    />\n");
    out
}

fn format_fill(fill: &Paint) -> String {
    match fill {
        Paint::None => "TRANSPARENT".to_string(),
        Paint::Color(color) => format_color(color),
    }
}

fn format_stroke(stroke: &StrokeStyle) -> String {
    let mut parts = vec![
        format!("color: {}", format_color(&stroke.color)),
        format!("width: {}px", format_number(stroke.width_px)),
    ];
    if let Some(cap) = &stroke.cap {
        parts.push(format!("cap: {cap}"));
    }
    if let Some(join) = &stroke.join {
        parts.push(format!("join: {join}"));
    }
    parts.join(" ")
}

fn format_color(color: &Color) -> String {
    format!("rgba({}, {}, {}, {})", color.r, color.g, color.b, color.a)
}

fn format_percent(value: f64) -> String {
    format!("{}%", format_number(value))
}

pub fn format_number(value: f64) -> String {
    let rounded = if value.abs() < 0.00005 { 0.0 } else { value };
    let mut rendered = format!("{rounded:.4}");
    while rendered.contains('.') && rendered.ends_with('0') {
        rendered.pop();
    }
    if rendered.ends_with('.') {
        rendered.pop();
    }
    rendered
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut out = String::with_capacity(digest.len() * 2);
    for byte in digest {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

pub fn component_name_from_path(path: &Path) -> String {
    let stem = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("Svg");
    let mut name = String::new();
    let mut capitalize = true;
    for ch in stem.chars() {
        if ch.is_ascii_alphanumeric() {
            if capitalize {
                name.push(ch.to_ascii_uppercase());
            } else {
                name.push(ch);
            }
            capitalize = false;
        } else {
            capitalize = true;
        }
    }
    if name.is_empty() {
        name.push_str("Svg");
    }
    if name
        .chars()
        .next()
        .map(|ch| ch.is_ascii_digit())
        .unwrap_or(false)
    {
        name.insert_str(0, "Svg");
    }
    name
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_basic_path_to_percent_pax() {
        let svg = r##"<svg viewBox="0 0 10 10"><path d="M0 0 L10 10 Z" fill="#f00"/></svg>"##;
        let import = import_svg(svg, Path::new("basic.svg"), "hash").unwrap();

        assert!(import.pax_source.contains("PathElement::Point(0%, 0%)"));
        assert!(import.pax_source.contains("PathElement::Point(100%, 100%)"));
        assert!(import.pax_source.contains("fill=rgba(255, 0, 0, 255)"));
    }

    #[test]
    fn converts_polygon_to_closed_percent_path() {
        let svg = r##"
            <svg viewBox="0 0 20 10">
                <polygon points="0,0 20,0 10,10" fill="#f00"/>
            </svg>
        "##;
        let import = import_svg(svg, Path::new("polygon.svg"), "hash").unwrap();

        assert_eq!(import.svg_path_count, 0);
        assert_eq!(import.svg_polygon_count, 1);
        assert_eq!(import.generated_path_count, 1);
        assert!(import.pax_source.contains("PathElement::Point(0%, 0%)"));
        assert!(import.pax_source.contains("PathElement::Point(100%, 0%)"));
        assert!(import.pax_source.contains("PathElement::Point(50%, 100%)"));
        assert!(import.pax_source.contains("PathElement::Close"));
        assert!(!import
            .warnings
            .iter()
            .any(|warning| warning.contains("<polygon>")));
    }

    #[test]
    fn removes_degenerate_line_segments_after_output_quantization() {
        let svg = r##"
            <svg viewBox="0 0 10 10">
                <path d="M0 0 h0 L10 0 v0 L10 10 Z"/>
            </svg>
        "##;
        let import = import_svg(svg, Path::new("degenerate-lines.svg"), "hash").unwrap();

        assert_eq!(import.pax_source.matches("PathElement::Line").count(), 2);
        assert_eq!(
            import
                .pax_source
                .matches("PathElement::Point(0%, 0%)")
                .count(),
            1
        );
        assert_eq!(
            import
                .pax_source
                .matches("PathElement::Point(100%, 0%)")
                .count(),
            1
        );
    }

    #[test]
    fn canonicalizes_repeated_polygon_vertices() {
        let svg = r##"
            <svg viewBox="0 0 10 10">
                <polygon points="0,0 10,0 10,0 10,10 0,0"/>
            </svg>
        "##;
        let import = import_svg(svg, Path::new("repeated-polygon.svg"), "hash").unwrap();

        assert_eq!(import.pax_source.matches("PathElement::Line").count(), 2);
        assert_eq!(
            import
                .pax_source
                .matches("PathElement::Point(0%, 0%)")
                .count(),
            1
        );
        assert!(import.pax_source.contains("PathElement::Close"));
    }

    #[test]
    fn applies_inherited_style_and_transform_to_polygon() {
        let svg = r##"
            <svg viewBox="0 0 20 20">
                <g fill="none" stroke="#000" stroke-width="2" transform="translate(5 0)">
                    <polygon points="0,0 5,0 5,5" transform="scale(2)"/>
                </g>
            </svg>
        "##;
        let import = import_svg(svg, Path::new("polygon-transform.svg"), "hash").unwrap();

        assert!(import.pax_source.contains("PathElement::Point(25%, 0%)"));
        assert!(import.pax_source.contains("PathElement::Point(75%, 0%)"));
        assert!(import.pax_source.contains("PathElement::Point(75%, 50%)"));
        assert!(import.pax_source.contains("width: 4px"));
        assert!(import.pax_source.contains("draw_start={draw_start}"));
    }

    #[test]
    fn preserves_document_order_across_paths_and_polygons() {
        let svg = r##"
            <svg viewBox="0 0 10 10">
                <path d="M0 0 L10 0" fill="none" stroke="#f00"/>
                <polygon points="0,0 10,0 5,10" fill="#00f"/>
            </svg>
        "##;
        let import = import_svg(svg, Path::new("order.svg"), "hash").unwrap();
        let polygon = import.pax_source.find("fill=rgba(0, 0, 255, 255)").unwrap();
        let path = import
            .pax_source
            .find("color: rgba(255, 0, 0, 255)")
            .unwrap();

        // Pax paints earlier siblings on top, so imported SVG shapes are emitted in reverse.
        assert!(polygon < path);
    }

    #[test]
    fn rejects_malformed_or_incomplete_polygon_points() {
        for (points, expected) in [
            ("0,0 10,0 nope 5,10", "malformed SVG polygon points"),
            ("0,0 10,0 5", "expected coordinate pairs"),
            ("0,0 10,0", "at least three coordinate pairs"),
            (
                "0,0 0,0 10,10 0,0",
                "at least three distinct rendered vertices",
            ),
        ] {
            let svg = format!(r##"<svg viewBox="0 0 10 10"><polygon points="{points}"/></svg>"##);
            let error = import_svg(&svg, Path::new("bad-polygon.svg"), "hash").unwrap_err();
            assert!(
                error.to_string().contains(expected),
                "unexpected error for `{points}`: {error}"
            );
        }
    }

    #[test]
    fn merges_adjacent_compatible_stroke_only_paths() {
        let svg = r##"
            <svg viewBox="0 0 10 10">
                <path d="M0 0 L10 0" fill="none" stroke="#000" stroke-width="2"/>
                <path d="M0 10 L10 10" fill="none" stroke="#000" stroke-width="2"/>
            </svg>
        "##;
        let import = import_svg(svg, Path::new("stroke.svg"), "hash").unwrap();

        assert_eq!(import.generated_path_count, 1);
        assert_eq!(import.pax_source.matches("<Path").count(), 1);
        assert!(import.pax_source.contains("draw_start={draw_start}"));
    }

    #[test]
    fn applies_simple_class_styles() {
        let svg = r##"
            <svg viewBox="0 0 10 10">
                <defs><style>.cls-1 { fill: none; stroke: #000; stroke-width: 5px; }</style></defs>
                <path class="cls-1" d="M0 0 L10 0"/>
            </svg>
        "##;
        let import = import_svg(svg, Path::new("class.svg"), "hash").unwrap();

        assert!(import.pax_source.contains("fill=TRANSPARENT"));
        assert!(import.pax_source.contains("width: 5px"));
    }

    #[test]
    fn applies_inherited_group_styles_and_opacity() {
        let svg = r##"
            <svg viewBox="0 0 10 10">
                <g fill="none" stroke="#000" stroke-width="2" opacity="0.5">
                    <path d="M0 0 L10 0"/>
                </g>
            </svg>
        "##;
        let import = import_svg(svg, Path::new("group.svg"), "hash").unwrap();

        assert!(import.pax_source.contains("fill=TRANSPARENT"));
        assert!(import.pax_source.contains("color: rgba(0, 0, 0, 128)"));
        assert!(import.pax_source.contains("width: 2px"));
    }

    #[test]
    fn flattens_path_transforms_into_percent_points() {
        let svg = r##"
            <svg viewBox="0 0 20 20">
                <g transform="translate(5 0)">
                    <path transform="scale(2)" d="M0 0 L5 5" fill="none" stroke="#000" stroke-width="2"/>
                </g>
            </svg>
        "##;
        let import = import_svg(svg, Path::new("transform.svg"), "hash").unwrap();

        assert!(import.pax_source.contains("PathElement::Point(25%, 0%)"));
        assert!(import.pax_source.contains("PathElement::Point(75%, 50%)"));
        assert!(import.pax_source.contains("width: 4px"));
    }

    #[test]
    fn rejects_arcs_until_they_are_flattened_to_curves() {
        let svg = r##"<svg viewBox="0 0 10 10"><path d="M0 0 A5 5 0 0 1 10 10"/></svg>"##;
        let error = import_svg(svg, Path::new("arc.svg"), "hash").unwrap_err();

        assert!(error.to_string().contains("unsupported SVG path command"));
    }

    #[test]
    fn rejects_unsupported_paint_instead_of_guessing() {
        let svg = r##"
            <svg viewBox="0 0 10 10">
                <path d="M0 0 L10 0" fill="url(#gradient)"/>
            </svg>
        "##;
        let error = import_svg(svg, Path::new("paint.svg"), "hash").unwrap_err();

        assert!(error.to_string().contains("unsupported fill paint"));
    }
}
