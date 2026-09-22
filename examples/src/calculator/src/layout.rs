//! Larger windows add LCD cells without magnifying the type or shrinking the keys.
pub const CELL: f64 = 9.6;
pub const LINE: f64 = 22.0;
pub const MIN_TEXT_ZOOM: i32 = -2;
pub const MAX_TEXT_ZOOM: i32 = 2;
pub const GUTTER: f64 = 22.0;
pub const KEYPAD_TOP: f64 = 331.0;
pub const KEYPAD_HEIGHT: f64 = 304.0;
pub const BODY_BELOW_SCREEN: f64 = KEYPAD_TOP + KEYPAD_HEIGHT + GUTTER + 1.0;

#[derive(Clone, Copy)]
pub struct TextGrid {
    pub font: f64,
    pub cell: f64,
    pub line: f64,
    pub columns: usize,
    pub rows: usize,
}
impl TextGrid {
    pub fn new(width: f64, height: f64, zoom: i32) -> Self {
        let font =
            [12., 14., 16., 20., 24.][(zoom.clamp(MIN_TEXT_ZOOM, MAX_TEXT_ZOOM) + 2) as usize];
        let cell = CELL * font / 16.;
        let line = LINE * font / 16.;
        Self {
            font,
            cell,
            line,
            columns: (((width - 28.) / cell).floor() as usize)
                .saturating_sub(2)
                .max(1),
            rows: ((height - 60.) / line).floor().max(1.) as usize,
        }
    }
    pub fn input_lines(self, characters: usize, screen_height: f64) -> usize {
        // Reserve a full history row, even at the largest text size on a short LCD.
        let maximum = ((screen_height - 76. - self.line) / self.line)
            .floor()
            .clamp(1., 3.) as usize;
        (characters / self.columns + 1).clamp(1, maximum)
    }
}

#[derive(Clone, Copy, Default, PartialEq)]
pub struct Layout {
    pub width: f64,
    pub height: f64,
    pub screen_width: f64,
    pub screen_height: f64,
    pub plot_width: f64,
    pub plot_height: f64,
    pub columns: usize,
    pub rows: usize,
    pub top: f64,
    pub page_height: f64,
}
impl Layout {
    pub fn for_window(w: f64, h: f64) -> Self {
        let width = (w - 24.)
            .min(420. + (w - 444.).max(0.) * 0.58)
            .clamp(296., 1280.);
        let screen_height = (h - BODY_BELOW_SCREEN - 48.).clamp(188., 1000.);
        let height = screen_height + BODY_BELOW_SCREEN;
        let screen_width = width - 56.;
        let top = ((h - height) * 0.5).max(24.);
        Self {
            width,
            height,
            screen_width,
            screen_height,
            plot_width: screen_width - 24.,
            plot_height: screen_height - 94.,
            columns: ((screen_width - 28.) / CELL).floor() as usize - 2,
            rows: ((screen_height - 60.) / LINE).floor() as usize,
            top,
            page_height: (height + 48.).max(h),
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn calculate_text_zoom_keeps_input_and_history_usable() {
        for width in [320., 390., 1440.] {
            let l = Layout::for_window(width, 700.);
            let small = TextGrid::new(l.screen_width - 12., l.screen_height, MIN_TEXT_ZOOM);
            let large = TextGrid::new(l.screen_width - 12., l.screen_height, MAX_TEXT_ZOOM);
            assert!(small.columns > large.columns);
            assert!(small.rows > large.rows);
            assert_eq!((small.font, large.font), (12., 24.));
            for zoom in MIN_TEXT_ZOOM..=MAX_TEXT_ZOOM {
                let g = TextGrid::new(l.screen_width - 12., l.screen_height, zoom);
                assert!((g.columns + 2) as f64 * g.cell <= l.screen_width - 40.);
                let input_height = g.input_lines(256, l.screen_height) as f64 * g.line;
                assert!(l.screen_height - 28. - input_height - 48. >= g.line);
            }
        }
    }
    #[test]
    fn resizing_adds_data_space() {
        let phone = Layout::for_window(390., 844.);
        let desktop = Layout::for_window(1440., 1080.);
        assert!(desktop.columns > phone.columns * 2);
        assert!(desktop.rows > phone.rows);
        assert!(desktop.plot_width > phone.plot_width);
        assert!(desktop.plot_height > phone.plot_height);
        for w in [320., 390., 600., 1440.] {
            let l = Layout::for_window(w, 700.);
            assert!((l.width - 44. - 32.) / 5. >= 44.);
            assert!(l.page_height >= l.height + 48.);
        }
        for w in 320..2400 {
            assert!(
                Layout::for_window(w as f64 + 1., 900.).width
                    >= Layout::for_window(w as f64, 900.).width
            );
        }
    }
}
