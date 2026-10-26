use pax_kit::*;

fn px(v: f64) -> Size {
    Size::Pixels(v.into())
}
pub fn point(x: f64, y: f64) -> PathElement {
    PathElement::Point(px(x), px(y))
}
pub fn line(path: &mut Vec<PathElement>, x: f64, y: f64, u: f64, v: f64) {
    path.extend([point(x, y), PathElement::Line, point(u, v)]);
}

/// Four cubic segments form each round lamp, with a separate closed contour.
pub fn disc(path: &mut Vec<PathElement>, x: f64, y: f64, r: f64) {
    let k = r * 0.5522847498;
    path.extend([
        point(x + r, y),
        PathElement::Cubic(px(x + r), px(y + k), px(x + k), px(y + r)),
        point(x, y + r),
        PathElement::Cubic(px(x - k), px(y + r), px(x - r), px(y + k)),
        point(x - r, y),
        PathElement::Cubic(px(x - r), px(y - k), px(x - k), px(y - r)),
        point(x, y - r),
        PathElement::Cubic(px(x + k), px(y - r), px(x + r), px(y - k)),
        point(x + r, y),
        PathElement::Close,
    ]);
}
