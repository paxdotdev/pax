use pax_text::OutlineFont;

fn main() -> Result<(), String> {
    const FONT: &[u8] = include_bytes!("../../pax-website/assets/fonts/Manrope.ttf");
    let mut font = OutlineFont::new(FONT, 800.0)?;
    let run = font.shape_line("portable")?;
    for cluster in &run.clusters {
        println!(
            "source bytes {}..{}, x={}, advance={}",
            cluster.byte_start, cluster.byte_end, cluster.x, cluster.advance
        );
    }
    Ok(())
}
