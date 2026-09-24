//! Larger windows add LCD cells without magnifying the type or shrinking the keys.
pub const CELL: f64 = 9.6;
pub const LINE: f64 = 22.0;
pub const MIN_TEXT_ZOOM: i32 = -2;
pub const MAX_TEXT_ZOOM: i32 = 2;
pub const GRAPH_TOP: f64 = 54.0;
pub const MOBILE_BREAKPOINT: f64 = 600.0;
pub const GUTTER: f64 = 22.0;
// Measure visible gutters from the key's outer bevel, not its hit rectangle.
pub const KEY_SIDE_OUTSET: f64 = 2.0;
pub const KEY_PAINT_BOTTOM: f64 = 13.5 + 31.5 + 3.2;
pub const CASE_FACE_SIDE_INSET: f64 = 7.0;
pub const CASE_FACE_BOTTOM_INSET: f64 = 13.0;
pub const KEY_TARGET: f64 = 47.0;
pub const KEY_PITCH: f64 = 51.0;
pub const UTILITY_TOP: f64 = 118.0;
pub const KEYPAD_TOP: f64 = UTILITY_TOP + 3.0 * KEY_PITCH;
pub const KEYPAD_HEIGHT: f64 = 4.0 * KEY_PITCH + KEY_TARGET;
pub const BODY_BELOW_SCREEN: f64 =
    KEYPAD_TOP + KEYPAD_HEIGHT + (KEY_PAINT_BOTTOM - KEY_TARGET) + GUTTER - KEY_SIDE_OUTSET;

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
    pub edge_to_edge: bool,
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
        Self::for_viewport(w, h, w < MOBILE_BREAKPOINT)
    }

    pub fn for_ios(w: f64, h: f64) -> Self {
        Self::for_viewport(w, h, true)
    }

    fn for_viewport(w: f64, h: f64, full_screen: bool) -> Self {
        let width = if full_screen {
            w.max(296.)
        } else {
            // Keep the LCD from shrinking as the window crosses the breakpoint.
            (420. + (w - 444.).max(0.) * 0.58)
                .clamp(MOBILE_BREAKPOINT, 1280.)
                .min(w)
        };
        let body_below_screen = BODY_BELOW_SCREEN
            + if full_screen {
                0.
            } else {
                CASE_FACE_BOTTOM_INSET - CASE_FACE_SIDE_INSET
            };
        let screen_height = if full_screen {
            (h - body_below_screen).max(188.)
        } else {
            (h - body_below_screen - 48.).clamp(188., 1000.)
        };
        let height = screen_height + body_below_screen;
        let screen_width = width - 56.;
        let top = if full_screen {
            0.
        } else {
            ((h - height) * 0.5).max(24.)
        };
        Self {
            edge_to_edge: full_screen,
            width,
            height,
            screen_width,
            screen_height,
            plot_width: screen_width - 24.,
            plot_height: screen_height - GRAPH_TOP - 18.,
            columns: ((screen_width - 28.) / CELL).floor() as usize - 2,
            rows: ((screen_height - 60.) / LINE).floor() as usize,
            top,
            page_height: (height + if full_screen { 0. } else { 48. }).max(h),
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn compact_keypad_keeps_touch_targets_and_gives_height_to_the_lcd() {
        assert!(KEY_TARGET >= 44.);
        assert!(KEY_PITCH - KEY_TARGET >= 4.);
        let phone = Layout::for_ios(440., 860.);
        assert!(phone.screen_height >= 310.);
        assert!(phone.plot_height >= 240.);
        assert_eq!(phone.height, 860.);
        let last_key_bottom = phone.screen_height + KEYPAD_TOP + 4. * KEY_PITCH + KEY_PAINT_BOTTOM;
        assert!((phone.height - last_key_bottom - (GUTTER - KEY_SIDE_OUTSET)).abs() < 1e-9);
    }
    #[test]
    fn mobile_windows_fill_the_surface_and_boxed_gutters_match() {
        for w in [320., 393., 599., 600., 601., 900.] {
            let l = Layout::for_window(w, 960.);
            assert_eq!(l.edge_to_edge, w < MOBILE_BREAKPOINT);
            if l.edge_to_edge {
                assert_eq!(l.width, w);
                assert_eq!(l.top, 0.);
                assert_eq!(l.page_height, 960.);
            } else {
                assert!(l.top >= 24.);
            }
            let side_inset = if l.edge_to_edge {
                0.
            } else {
                CASE_FACE_SIDE_INSET
            };
            let bottom_inset = if l.edge_to_edge {
                0.
            } else {
                CASE_FACE_BOTTOM_INSET
            };
            let side_gap = GUTTER - KEY_SIDE_OUTSET - side_inset;
            let key_bottom = l.screen_height + KEYPAD_TOP + 4. * KEY_PITCH + KEY_PAINT_BOTTOM;
            let bottom_gap = l.height - bottom_inset - key_bottom;
            assert!((bottom_gap - side_gap).abs() < 1e-9, "width {w}");
        }
        assert!(Layout::for_ios(1024., 1366.).edge_to_edge);
    }
    #[test]
    fn ios_fills_the_surface_without_shrinking_touch_targets() {
        for (w, h) in [(320., 568.), (440., 956.), (956., 440.), (1024., 1366.)] {
            let l = Layout::for_ios(w, h);
            assert_eq!(l.width, w);
            assert!(l.page_height >= h);
            assert_eq!(l.page_height, l.height);
            assert_eq!(l.top, 0.);
            assert!((l.width - 44. - 32.) / 5. >= 44.);
            assert!(l.screen_height >= 188.);
        }
        assert_eq!(Layout::for_ios(440., 956.).page_height, 956.);
        let safe = Layout::for_ios(440., 956. - 62. - 34.);
        assert_eq!(safe.page_height, 860.);
        assert!(safe.screen_height >= 188.);
    }
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
            assert!(l.page_height >= l.height + if l.edge_to_edge { 0. } else { 48. });
        }
        for w in 320..2400 {
            assert!(
                Layout::for_window(w as f64 + 1., 900.).width
                    >= Layout::for_window(w as f64, 900.).width
            );
        }
    }
}
