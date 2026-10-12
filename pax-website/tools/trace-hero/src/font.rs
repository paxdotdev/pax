//! Private font extraction for the fixed website hero, never linked by the app.
//!
//! Runtime text/API exploration is deferred to PAX-1036; the standalone prototype
//! is preserved at 79a522edbc29614cf39aae70d30fba680f3c4b86.
use harfrust::{font::Blob, Buffer, Direction, Font, Script, ShapeOptions, ShaperFont};
use skrifa::{
    instance::{LocationRef, Size as FontSize},
    outline::{DrawSettings, OutlinePen},
    FontRef, MetadataProvider,
};
use std::collections::HashMap;

/// One shaped line. Position clusters using their advances, not their ink bounds.
#[derive(Debug, Clone, PartialEq)]
pub struct OutlineRun {
    /// Logical clusters in this line's left-to-right order.
    pub clusters: Vec<OutlineCluster>,
    /// Total horizontal advance, in em units.
    pub advance: f64,
    /// Font ascent above the baseline, in em units.
    pub ascent: f64,
    /// Font descent, normally negative, in em units.
    pub descent: f64,
    /// Font cap height above the baseline, in em units.
    pub cap_height: f64,
}

/// An indivisible animation unit that may contain a ligature or combining marks.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct OutlineCluster {
    /// Inclusive UTF-8 byte offset in the source string.
    pub byte_start: usize,
    /// Exclusive UTF-8 byte offset in the source string.
    pub byte_end: usize,
    /// Font glyph IDs, for inspection; repeated IDs are not occurrence keys.
    pub glyph_ids: Vec<u32>,
    /// Horizontal position of the cluster origin in the run, in em units.
    pub x: f64,
    /// Horizontal advance, in em units; may exceed the visible outline.
    pub advance: f64,
    /// Cluster-local contours, baseline y=0, positive y downward, in em units.
    pub outline: Vec<OutlineCommand>,
}

/// A Bézier contour command with exact, unhinted em-space coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OutlineCommand {
    /// Start a contour at x, y.
    Move(f64, f64),
    /// Draw a line to x, y.
    Line(f64, f64),
    /// Draw a quadratic curve through a control point to an endpoint.
    Quad(f64, f64, f64, f64),
    /// Draw a cubic curve through two control points to an endpoint.
    Cubic(f64, f64, f64, f64, f64, f64),
    /// Close the current contour. Preserve winding when filling it.
    Close,
}

/// A font instance with an owned outline cache, independent of any Pax component.
///
/// Font bytes are owned or static, and shared between shaping and outlining.
/// The fixed face, weight and unhinted outline mode define the cache identity. Drop the helper
/// after prewarming fixed runs, or retain it when shaping changing text.
pub struct OutlineFont {
    bytes: Blob,
    font: Font,
    outlines: HashMap<u32, Vec<Command>>,
    units: f64,
    ascent: f64,
    descent: f64,
    cap_height: f64,
}

impl OutlineFont {
    /// Opens face zero at the requested `wght` coordinate.
    ///
    /// Accepts owned `Vec<u8>` or static `&'static [u8]` without copying font bytes.
    /// Requires a variable TrueType/OpenType font with a weight axis and cap-height
    /// metrics. Rejects invalid bytes, non-finite weights, and weights outside the axis.
    pub fn new(bytes: impl Into<Blob>, weight: f32) -> Result<Self, String> {
        let bytes = bytes.into();
        let face = FontRef::new(bytes.as_ref()).map_err(|e| format!("Invalid font: {e}"))?;
        let axis = face
            .axes()
            .iter()
            .find(|a| a.tag() == skrifa::Tag::new(b"wght"))
            .ok_or("Font has no weight axis")?;
        if !weight.is_finite() || !(axis.min_value()..=axis.max_value()).contains(&weight) {
            return Err("Font weight is outside its supported range".into());
        }
        let font = Font::new(bytes.clone(), 0)
            .ok_or("HarfRust could not read the font")?
            .instance_builder()
            .variations([("wght", weight)])
            .build();
        // Use HarfRust's normalized coordinates for outlines and metrics too.
        let metrics = face.metrics(
            FontSize::unscaled(),
            LocationRef::new(font.normalized_coords()),
        );
        let units = metrics.units_per_em as f64;
        if units == 0.0 {
            return Err("Font has zero units per em".into());
        }
        let ascent = metrics.ascent as f64 / units;
        let descent = metrics.descent as f64 / units;
        Ok(Self {
            bytes,
            font,
            outlines: HashMap::new(),
            units,
            // Keep baseline placement a caller decision.
            ascent,
            descent,
            cap_height: metrics.cap_height.ok_or("Font has no cap height")? as f64 / units,
        })
    }

    /// Shapes one Latin LTR line, preserving kerning, ligatures and cluster ranges.
    ///
    /// Coordinates are in em units with a baseline at y=0 and positive y downward.
    /// Rejects unsupported characters or missing glyphs; no wrapping, bidi, fallback,
    /// font discovery, native selection, or rendering is performed. Spaces retain
    /// their advance with an empty outline. Cached outlines live with this font.
    pub fn shape_line(&mut self, text: &str) -> Result<OutlineRun, String> {
        // This helper deliberately has no bidi, fallback or paragraph machinery.
        if text.chars().any(|c| {
            !((' '..='~').contains(&c)
                || ('\u{00a0}'..='\u{024f}').contains(&c)
                || ('\u{0300}'..='\u{036f}').contains(&c))
        }) {
            return Err("Outlines support a single Latin LTR line only".into());
        }
        let mut buffer = Buffer::new();
        buffer.push_str(text);
        buffer.set_direction(Direction::LeftToRight);
        buffer.set_script(Some(Script::LATIN));
        harfrust::shape(
            &ShaperFont::new(&self.font),
            &mut buffer,
            ShapeOptions::default(),
        )
        .map_err(|e| format!("Could not shape text: {e:?}"))?;
        let face = FontRef::new(self.bytes.as_ref()).map_err(|e| format!("Invalid font: {e}"))?;
        let outlines = face.outline_glyphs();
        let location = LocationRef::new(self.font.normalized_coords());
        let mut clusters: Vec<OutlineCluster> = Vec::new();
        let mut pen_x = 0.0;
        for (info, pos) in buffer.glyph_infos().iter().zip(buffer.glyph_positions()) {
            if info.glyph_id == 0 {
                return Err(format!("Missing glyph at byte {}", info.cluster));
            }
            if clusters
                .last()
                .is_none_or(|c| c.byte_start != info.cluster as usize)
            {
                if let Some(previous) = clusters.last_mut() {
                    previous.byte_end = info.cluster as usize;
                }
                clusters.push(OutlineCluster {
                    byte_start: info.cluster as usize,
                    byte_end: text.len(),
                    x: pen_x,
                    ..Default::default()
                });
            }
            if let std::collections::hash_map::Entry::Vacant(entry) =
                self.outlines.entry(info.glyph_id)
            {
                let glyph = outlines
                    .get(skrifa::GlyphId::new(info.glyph_id))
                    .ok_or_else(|| format!("No outline for glyph {}", info.glyph_id))?;
                let mut pen = Pen::default();
                glyph
                    .draw(
                        DrawSettings::unhinted(FontSize::unscaled(), location),
                        &mut pen,
                    )
                    .map_err(|e| format!("Could not draw glyph: {e}"))?;
                entry.insert(pen.0);
            }
            let cluster = clusters.last_mut().ok_or("Missing shaping cluster")?;
            let x = pen_x - cluster.x + pos.x_offset as f64 / self.units;
            let y = pos.y_offset as f64 / self.units;
            for command in &self.outlines[&info.glyph_id] {
                command.append(&mut cluster.outline, self.units, x, y);
            }
            cluster.glyph_ids.push(info.glyph_id);
            let advance = pos.x_advance as f64 / self.units;
            cluster.advance += advance;
            pen_x += advance;
        }
        Ok(OutlineRun {
            clusters,
            advance: pen_x,
            ascent: self.ascent,
            descent: self.descent,
            cap_height: self.cap_height,
        })
    }
}

#[derive(Debug, PartialEq)]
enum Command {
    Move(f32, f32),
    Line(f32, f32),
    Quad(f32, f32, f32, f32),
    Cubic(f32, f32, f32, f32, f32, f32),
    Close,
}

impl Command {
    fn append(&self, out: &mut Vec<OutlineCommand>, units: f64, dx: f64, dy: f64) {
        let x = |v: f32| v as f64 / units + dx;
        let y = |v: f32| -(v as f64 / units + dy);
        out.push(match *self {
            Self::Move(a, b) => OutlineCommand::Move(x(a), y(b)),
            Self::Line(a, b) => OutlineCommand::Line(x(a), y(b)),
            Self::Quad(a, b, c, d) => OutlineCommand::Quad(x(a), y(b), x(c), y(d)),
            Self::Cubic(a, b, c, d, e, f) => {
                OutlineCommand::Cubic(x(a), y(b), x(c), y(d), x(e), y(f))
            }
            Self::Close => OutlineCommand::Close,
        });
    }
}

#[derive(Default)]
struct Pen(Vec<Command>);
impl OutlinePen for Pen {
    fn move_to(&mut self, x: f32, y: f32) {
        self.0.push(Command::Move(x, y));
    }
    fn line_to(&mut self, x: f32, y: f32) {
        self.0.push(Command::Line(x, y));
    }
    fn quad_to(&mut self, a: f32, b: f32, x: f32, y: f32) {
        self.0.push(Command::Quad(a, b, x, y));
    }
    fn curve_to(&mut self, a: f32, b: f32, c: f32, d: f32, x: f32, y: f32) {
        self.0.push(Command::Cubic(a, b, c, d, x, y));
    }
    fn close(&mut self) {
        self.0.push(Command::Close);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const FONT: &[u8] = include_bytes!("../../../assets/fonts/Manrope.ttf");
    const WORDS: [&str; 3] = ["high-performance", "native", "portable"];

    #[test]
    fn pinned_hero_preserves_advances_and_reuses_outlines() {
        let mut font = OutlineFont::new(FONT, 800.0).unwrap();
        for ((text, count), advance) in WORDS.into_iter().zip([16, 6, 8]).zip([8.962, 3.115, 4.218])
        {
            let run = font.shape_line(text).unwrap();
            assert_eq!(run.clusters.len(), count);
            assert!((run.advance - advance).abs() < 0.00001);
            assert!(run.clusters.iter().all(|c| !c.outline.is_empty()));
        }
        assert_eq!(font.outlines.len(), 17);
        for text in WORDS {
            font.shape_line(text).unwrap();
        }
        assert_eq!(font.outlines.len(), 17);
    }

    #[test]
    fn shaping_keeps_kerning_ligatures_accents_and_spaces() {
        let mut font = OutlineFont::new(FONT, 800.0).unwrap();
        let pair = font.shape_line("AV").unwrap().advance;
        assert!(
            pair < font.shape_line("A").unwrap().advance + font.shape_line("V").unwrap().advance
        );
        let run = font.shape_line("office").unwrap();
        assert_eq!(
            run.clusters
                .iter()
                .map(|c| (c.byte_start, c.byte_end))
                .collect::<Vec<_>>(),
            [(0, 1), (1, 3), (3, 4), (4, 5), (5, 6)]
        );
        assert_eq!(
            font.shape_line("e\u{0301}").unwrap().clusters[0].outline,
            font.shape_line("é").unwrap().clusters[0].outline
        );
        let spaced = font.shape_line("a a").unwrap();
        assert!(spaced.clusters[1].outline.is_empty());
        assert!(spaced.clusters[1].advance > 0.0);
        assert_ne!(spaced.clusters[0].byte_start, spaced.clusters[2].byte_start);
        // This font supports composed é, but has no standalone acute mark for x.
        assert!(
            matches!(font.shape_line("x\u{0301}"), Err(error) if error.contains("Missing glyph"))
        );
    }

    #[test]
    fn rejects_out_of_scope_input_and_checks_axis() {
        assert!(OutlineFont::new(&[] as &'static [u8], 800.0).is_err());
        for weight in [900.0, f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            assert!(OutlineFont::new(FONT, weight).is_err());
        }
        let mut font = OutlineFont::new(FONT, 800.0).unwrap();
        for text in ["two\nlines", "a\tb", "مرحبا", "🦀"] {
            assert!(font.shape_line(text).is_err());
        }
        let light = OutlineFont::new(FONT, 200.0)
            .unwrap()
            .shape_line("H")
            .unwrap();
        let bold = font.shape_line("H").unwrap();
        assert_ne!(light.clusters[0].outline, bold.clusters[0].outline);
    }
}
