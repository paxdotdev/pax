//! Fixed-font, single-line Latin shaping for the website hero, not a text-layout API.
use crate::HeroCluster;
use harfrust::{Buffer, Direction, Font, Script, ShapeOptions, ShaperFont};
use pax_kit::{PathElement, Size};
use skrifa::{
    instance::{LocationRef, Size as FontSize},
    outline::{DrawSettings, OutlinePen},
    FontRef, MetadataProvider,
};
use std::collections::HashMap;

const FONT: &[u8] = include_bytes!("../assets/fonts/Manrope.ttf");
pub const WORDS: [&str; 3] = ["high-performance", "native", "portable"];

pub struct HeroRun {
    pub clusters: Vec<HeroCluster>,
    pub advance: f64,
    pub baseline: f64,
    pub cap_height: f64,
}

// Cache ownership fixes font, face, axis coordinates and outline settings. It is
// dropped after prewarming the three runs; only their immutable Pax data survives.
struct HeroFont {
    bytes: &'static [u8],
    font: Font,
    outlines: HashMap<u32, Vec<Command>>,
    units: f64,
    baseline: f64,
    cap_height: f64,
}

impl HeroFont {
    fn new(bytes: &'static [u8], weight: f32) -> Result<Self, String> {
        let face = FontRef::new(bytes).map_err(|e| format!("Invalid hero font: {e}"))?;
        let axis = face
            .axes()
            .iter()
            .find(|a| a.tag() == skrifa::Tag::new(b"wght"))
            .ok_or("Hero font has no weight axis")?;
        if !weight.is_finite() || !(axis.min_value()..=axis.max_value()).contains(&weight) {
            return Err("Hero font weight is outside its supported range".into());
        }
        let font = Font::new(bytes, 0)
            .ok_or("HarfRust could not read the hero font")?
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
            return Err("Hero font has zero units per em".into());
        }
        let ascent = metrics.ascent as f64 / units;
        let descent = metrics.descent as f64 / units;
        Ok(Self {
            bytes,
            font,
            outlines: HashMap::new(),
            units,
            // Center the font's line metrics in the existing 1.5em signboard.
            baseline: (1.5 - (ascent - descent)) / 2.0 + ascent,
            cap_height: metrics.cap_height.ok_or("Hero font has no cap height")? as f64 / units,
        })
    }

    fn shape(&mut self, text: &str) -> Result<HeroRun, String> {
        // This helper deliberately has no bidi, fallback or paragraph machinery.
        if text.chars().any(|c| {
            !((' '..='~').contains(&c)
                || ('\u{00a0}'..='\u{024f}').contains(&c)
                || ('\u{0300}'..='\u{036f}').contains(&c))
        }) {
            return Err("Hero outlines support a single Latin LTR line only".into());
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
        .map_err(|e| format!("Could not shape hero text: {e:?}"))?;
        let face = FontRef::new(self.bytes).map_err(|e| format!("Invalid hero font: {e}"))?;
        let outlines = face.outline_glyphs();
        let location = LocationRef::new(self.font.normalized_coords());
        let mut clusters: Vec<HeroCluster> = Vec::new();
        let mut pen_x = 0.0;
        for (info, pos) in buffer.glyph_infos().iter().zip(buffer.glyph_positions()) {
            if info.glyph_id == 0 {
                return Err(format!("Missing hero glyph at byte {}", info.cluster));
            }
            if clusters
                .last()
                .is_none_or(|c| c.byte_start != info.cluster as usize)
            {
                if let Some(previous) = clusters.last_mut() {
                    previous.byte_end = info.cluster as usize;
                }
                clusters.push(HeroCluster {
                    byte_start: info.cluster as usize,
                    byte_end: text.len(),
                    ordinal: clusters.len(),
                    x: pen_x,
                    ..Default::default()
                });
            }
            if let std::collections::hash_map::Entry::Vacant(entry) =
                self.outlines.entry(info.glyph_id)
            {
                let glyph = outlines
                    .get(skrifa::GlyphId::new(info.glyph_id))
                    .ok_or_else(|| format!("No outline for hero glyph {}", info.glyph_id))?;
                let mut pen = Pen::default();
                glyph
                    .draw(
                        DrawSettings::unhinted(FontSize::unscaled(), location),
                        &mut pen,
                    )
                    .map_err(|e| format!("Could not draw hero glyph: {e}"))?;
                entry.insert(pen.0);
            }
            let cluster = clusters.last_mut().ok_or("Missing shaping cluster")?;
            let x = pen_x - cluster.x + pos.x_offset as f64 / self.units;
            let y = pos.y_offset as f64 / self.units;
            for command in &self.outlines[&info.glyph_id] {
                command.append(&mut cluster.elements, self.units, x, y);
            }
            cluster.glyph_ids.push(info.glyph_id);
            let advance = pos.x_advance as f64 / self.units;
            cluster.advance += advance;
            pen_x += advance;
        }
        Ok(HeroRun {
            clusters,
            advance: pen_x,
            baseline: self.baseline,
            cap_height: self.cap_height,
        })
    }
}

pub fn shape_hero() -> Result<[HeroRun; 3], String> {
    let mut font = HeroFont::new(FONT, 800.0)?;
    Ok([
        font.shape(WORDS[0])?,
        font.shape(WORDS[1])?,
        font.shape(WORDS[2])?,
    ])
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
    fn append(&self, out: &mut Vec<PathElement>, units: f64, dx: f64, dy: f64) {
        let x = |v: f32| Size::Percent(((v as f64 / units + dx) * 100.0).into());
        let y = |v: f32| Size::Percent((-(v as f64 / units + dy) * 100.0).into());
        match *self {
            Self::Move(a, b) => out.push(PathElement::Point(x(a), y(b))),
            Self::Line(a, b) => {
                out.push(PathElement::Line);
                out.push(PathElement::Point(x(a), y(b)));
            }
            Self::Quad(a, b, c, d) => {
                out.push(PathElement::Quadratic(x(a), y(b)));
                out.push(PathElement::Point(x(c), y(d)));
            }
            Self::Cubic(a, b, c, d, e, f) => {
                out.push(PathElement::Cubic(x(a), y(b), x(c), y(d)));
                out.push(PathElement::Point(x(e), y(f)));
            }
            Self::Close => out.push(PathElement::Close),
        }
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

    #[test]
    fn pinned_hero_preserves_advances_and_reuses_outlines() {
        let mut font = HeroFont::new(FONT, 800.0).unwrap();
        for ((text, count), advance) in WORDS.into_iter().zip([16, 6, 8]).zip([8.962, 3.115, 4.218])
        {
            let run = font.shape(text).unwrap();
            assert_eq!(run.clusters.len(), count);
            assert!((run.advance - advance).abs() < 0.00001);
            assert!(run.clusters.iter().all(|c| !c.elements.is_empty()));
        }
        assert_eq!(font.outlines.len(), 17);
        for text in WORDS {
            font.shape(text).unwrap();
        }
        assert_eq!(font.outlines.len(), 17);
    }

    #[test]
    fn shaping_keeps_kerning_ligatures_accents_and_spaces() {
        let mut font = HeroFont::new(FONT, 800.0).unwrap();
        let pair = font.shape("AV").unwrap().advance;
        assert!(pair < font.shape("A").unwrap().advance + font.shape("V").unwrap().advance);
        let run = font.shape("office").unwrap();
        assert_eq!(
            run.clusters
                .iter()
                .map(|c| (c.byte_start, c.byte_end))
                .collect::<Vec<_>>(),
            [(0, 1), (1, 3), (3, 4), (4, 5), (5, 6)]
        );
        assert_eq!(
            font.shape("e\u{0301}").unwrap().clusters[0].elements,
            font.shape("é").unwrap().clusters[0].elements
        );
        let spaced = font.shape("a a").unwrap();
        assert!(spaced.clusters[1].elements.is_empty());
        assert!(spaced.clusters[1].advance > 0.0);
        assert_ne!(spaced.clusters[0].byte_start, spaced.clusters[2].byte_start);
        // This font supports composed é, but has no standalone acute mark for x.
        assert!(
            matches!(font.shape("x\u{0301}"), Err(error) if error.contains("Missing hero glyph"))
        );
    }

    #[test]
    fn rejects_out_of_scope_input_and_checks_axis() {
        assert!(HeroFont::new(&[], 800.0).is_err());
        assert!(HeroFont::new(FONT, 900.0).is_err());
        let mut font = HeroFont::new(FONT, 800.0).unwrap();
        for text in ["two\nlines", "a\tb", "مرحبا", "🦀"] {
            assert!(font.shape(text).is_err());
        }
        let light = HeroFont::new(FONT, 200.0).unwrap().shape("H").unwrap();
        let bold = font.shape("H").unwrap();
        assert_ne!(light.clusters[0].elements, bold.clusters[0].elements);
    }
}
