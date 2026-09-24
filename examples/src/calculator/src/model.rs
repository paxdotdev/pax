//! Calculator editing and history, independent of rendering and platform events.

use crate::expression::{format_number, Error, Expression, MAX_INPUT};
use crate::graph::{zoom_scale, MAX_ZOOM, MIN_ZOOM};
use crate::layout::{MAX_TEXT_ZOOM, MIN_TEXT_ZOOM};

pub const DEFAULT_POLAR_SOURCE: &str = "1+3*cos(11*θ)";
const DEFAULT_POLAR_RADIUS: f64 = 4.0;

/// Secondary legends and actions are shared by the keypad and its state tests.
pub fn secondary_key(action: &str) -> (&'static str, &'static str) {
    match action {
        "sin" => ("asin", "asin"),
        "cos" => ("acos", "acos"),
        "tan" => ("atan", "atan"),
        "ln" => ("eˣ", "exp"),
        "log" => ("10ˣ", "pow10"),
        "sqrt" => ("x²", "square"),
        "^" => ("1/x", "reciprocal"),
        "π" => ("e", "e"),
        "variable" => ("MODE", "mode"),
        _ => ("", ""),
    }
}

#[derive(Clone, Default)]
pub struct Buffer {
    pub text: String,
    pub cursor: usize,
    pub error: Option<Error>,
}

impl Buffer {
    pub fn new(text: &str) -> Self {
        Self {
            text: text.into(),
            cursor: text.chars().count(),
            error: None,
        }
    }
    fn edit(&mut self, insertion: &str, delete_before: bool, delete_after: bool) {
        let mut chars: Vec<_> = self.text.chars().collect();
        self.cursor = self.cursor.min(chars.len());
        if delete_before && self.cursor > 0 {
            self.cursor -= 1;
            chars.remove(self.cursor);
        }
        if delete_after && self.cursor < chars.len() {
            chars.remove(self.cursor);
        }
        let inserted: Vec<_> = insertion.chars().collect();
        if chars.len() + inserted.len() > MAX_INPUT {
            self.error = Some(Error {
                message: "Limit: 256 characters".into(),
                start: self.cursor,
                end: self.cursor + 1,
            });
            return;
        }
        chars.splice(self.cursor..self.cursor, inserted.iter().copied());
        self.cursor += inserted.len();
        self.text = chars.into_iter().collect();
        self.error = None;
    }
}

#[derive(Clone)]
pub struct Entry {
    pub expression: String,
    pub result: String,
}

pub struct Model {
    pub calculate: Buffer,
    pub graph: Buffer,
    pub graph_mode: bool,
    pub opening_view: bool,
    pub second: bool,
    pub polar: bool,
    alternate_graph: Buffer,
    alternate_plotted: Expression,
    alternate_source: String,
    pub graph_editing: bool,
    pub zoom_level: i32,
    pub text_zoom_level: i32,
    pub history_follow_requested: bool,
    pub history: Vec<Entry>,
    pub answer: Option<f64>,
    pub plotted: Expression,
    pub plotted_source: String,
    pub dirty: bool,
    pub plot_dirty: bool,
    pub home_requested: bool,
    pub pan: (f64, f64),
    fresh: bool,
    history_index: Option<usize>,
    history_draft: Buffer,
    repeat: Option<String>,
}

impl Default for Model {
    fn default() -> Self {
        Self {
            calculate: Buffer::default(),
            graph: Buffer::new(DEFAULT_POLAR_SOURCE),
            graph_mode: true,
            opening_view: true,
            second: false,
            polar: true,
            alternate_graph: Buffer::new("sin(x)"),
            alternate_plotted: Expression::parse("sin(x)", true, None).unwrap(),
            alternate_source: "sin(x)".into(),
            graph_editing: false,
            zoom_level: 0,
            text_zoom_level: 0,
            history_follow_requested: false,
            history: Vec::new(),
            answer: None,
            plotted: Expression::parse_polar(DEFAULT_POLAR_SOURCE, None).unwrap(),
            plotted_source: DEFAULT_POLAR_SOURCE.into(),
            dirty: true,
            plot_dirty: true,
            home_requested: true,
            pan: (0., 0.),
            fresh: false,
            history_index: None,
            history_draft: Buffer::default(),
            repeat: None,
        }
    }
}

impl Model {
    pub fn fit_opening_view(&mut self, width: f64, height: f64) {
        if !self.opening_view || !width.is_finite() || !height.is_finite() {
            return;
        }
        // Frame the whole flower while initial bounds and native safe-area
        // insets settle. The first interaction hands zoom back to the user.
        let diameter = width.min(height) * 0.95;
        self.zoom_level = (MIN_ZOOM..=MAX_ZOOM)
            .rev()
            .find(|&level| 2. * DEFAULT_POLAR_RADIUS * zoom_scale(level) <= diameter)
            .unwrap_or(MIN_ZOOM);
        self.home_requested = true;
        self.dirty = true;
        self.plot_dirty = true;
    }

    pub fn history_rows(&self, columns: usize) -> Vec<(String, bool)> {
        let columns = columns.max(1);
        let mut rows = Vec::new();
        for entry in &self.history {
            if !rows.is_empty() {
                rows.push((String::new(), true));
            }
            let chars: Vec<_> = entry.expression.chars().collect();
            for (i, chunk) in chars.chunks(columns).enumerate() {
                rows.push((
                    format!(
                        "{}{}",
                        if i == 0 { "› " } else { "  " },
                        chunk.iter().collect::<String>()
                    ),
                    true,
                ));
            }
            let chars: Vec<_> = entry.result.chars().collect();
            for chunk in chars.chunks(columns + 2) {
                rows.push((
                    format!(
                        "{:>width$}",
                        chunk.iter().collect::<String>(),
                        width = columns + 2
                    ),
                    false,
                ));
            }
        }
        rows
    }

    pub fn buffer(&self) -> &Buffer {
        if self.graph_mode {
            &self.graph
        } else {
            &self.calculate
        }
    }
    pub fn buffer_mut(&mut self) -> &mut Buffer {
        if self.graph_mode {
            &mut self.graph
        } else {
            &mut self.calculate
        }
    }

    /// Consume 2nd once per on-screen key. Hardware text entry is never remapped.
    pub fn press_key(&mut self, primary: &str, secondary: &str) {
        self.opening_view = false;
        if primary == "second" {
            self.second = !self.second;
            self.dirty = true;
            return;
        }
        let action = if self.second && !secondary.is_empty() {
            secondary
        } else {
            primary
        };
        self.action(action);
    }

    pub fn action(&mut self, action: &str) {
        self.opening_view = false;
        self.second = false;
        self.dirty = true;
        match action {
            "calculate" => {
                self.graph_mode = false;
            }
            "graph" => {
                if self.graph_mode {
                    self.execute();
                }
                self.home_requested = true;
                self.graph_editing = true;
                self.graph_mode = true;
                self.plot_dirty = true;
            }
            "mode" => {
                if self.graph_mode {
                    self.polar = !self.polar;
                    std::mem::swap(&mut self.graph, &mut self.alternate_graph);
                    std::mem::swap(&mut self.plotted, &mut self.alternate_plotted);
                    std::mem::swap(&mut self.plotted_source, &mut self.alternate_source);
                    self.graph_editing = true;
                    self.home_requested = true;
                    self.plot_dirty = true;
                }
            }
            "zoom-in" | "zoom-out" => {
                let delta = if action == "zoom-in" { 1 } else { -1 };
                if self.graph_mode {
                    self.zoom_level = (self.zoom_level + delta).clamp(MIN_ZOOM, MAX_ZOOM);
                    self.graph_editing = false;
                    self.plot_dirty = true;
                } else {
                    self.text_zoom_level =
                        (self.text_zoom_level + delta).clamp(MIN_TEXT_ZOOM, MAX_TEXT_ZOOM);
                }
            }
            "switch" => {
                self.graph_mode = !self.graph_mode;
                self.plot_dirty = true;
            }
            "edit" => {
                self.graph_editing = self.graph_mode;
                self.fresh = false;
            }
            "escape" => {
                if self.graph_mode {
                    self.graph_editing = false;
                } else {
                    self.calculate.error = None;
                }
            }
            "clear" => {
                *self.buffer_mut() = Buffer::default();
                self.graph_editing = self.graph_mode;
                self.fresh = false;
                self.history_index = None;
            }
            "backspace" | "delete" => {
                self.buffer_mut()
                    .edit("", action == "backspace", action == "delete");
                self.graph_editing = self.graph_mode;
                self.fresh = false;
            }
            "home" if self.graph_mode && !self.graph_editing => {
                self.home_requested = true;
            }
            "home" => {
                self.buffer_mut().cursor = 0;
                self.fresh = false;
            }
            "end" => {
                self.buffer_mut().cursor = self.buffer().text.chars().count();
                self.fresh = false;
            }
            "left" | "right" | "up" | "down" if self.graph_mode && !self.graph_editing => {
                match action {
                    "left" => self.pan.0 -= 48.,
                    "right" => self.pan.0 += 48.,
                    "up" => self.pan.1 -= 48.,
                    _ => self.pan.1 += 48.,
                }
            }
            "left" => {
                self.buffer_mut().cursor = self.buffer().cursor.saturating_sub(1);
                self.fresh = false;
            }
            "right" => {
                self.buffer_mut().cursor =
                    (self.buffer().cursor + 1).min(self.buffer().text.chars().count());
                self.fresh = false;
            }
            "up" | "down" if !self.graph_mode => self.recall(action == "up"),
            "up" | "down" => (),
            "enter" => self.execute(),
            value => {
                let value = match value {
                    "variable" => {
                        if self.polar {
                            "θ"
                        } else {
                            "x"
                        }
                    }
                    "sin" => "sin(",
                    "cos" => "cos(",
                    "tan" => "tan(",
                    "sec" => "sec(",
                    "csc" => "csc(",
                    "asin" => "asin(",
                    "acos" => "acos(",
                    "atan" => "atan(",
                    "exp" => "e^(",
                    "pow10" => "10^(",
                    "ln" => "ln(",
                    "log" => "log(",
                    "sqrt" => "sqrt(",
                    "square" => "^2",
                    "reciprocal" => "^(-1)",
                    other => other,
                };
                if matches!(value, "x" | "θ") && !self.graph_mode {
                    return;
                }
                if self.fresh && !self.graph_mode {
                    let prefix = if matches!(
                        value,
                        "+" | "-" | "−" | "×" | "÷" | "*" | "/" | "^" | "^2" | "^(-1)"
                    ) && self.answer.is_some()
                    {
                        "ans"
                    } else {
                        ""
                    };
                    self.calculate = Buffer::new(prefix);
                }
                self.buffer_mut().edit(value, false, false);
                self.graph_editing = self.graph_mode;
                self.fresh = false;
                self.history_index = None;
            }
        }
    }

    fn execute(&mut self) {
        if self.buffer().text.trim().is_empty() {
            if self.graph_mode {
                return;
            }
            let Some(repeat) = &self.repeat else {
                return;
            };
            self.calculate = Buffer::new(repeat);
        }
        let text = self.buffer().text.clone();
        let parsed = if self.graph_mode && self.polar {
            Expression::parse_polar(&text, self.answer)
        } else {
            Expression::parse(&text, self.graph_mode, self.answer)
        };
        let expression = match parsed {
            Ok(e) => e,
            Err(e) => {
                self.fail(e);
                return;
            }
        };
        if self.graph_mode {
            self.plotted = expression;
            self.plotted_source = text;
            self.graph.error = None;
            self.graph_editing = false;
            self.plot_dirty = true;
        } else {
            match expression.evaluate(0.) {
                Ok(result) => {
                    self.repeat = Some(expression.repeat_source(&text));
                    self.answer = Some(result);
                    self.history_follow_requested = true;
                    self.history.push(Entry {
                        expression: text,
                        result: format_number(result),
                    });
                    if self.history.len() > 20 {
                        self.history.remove(0);
                    }
                    self.calculate = Buffer::default();
                    self.fresh = true;
                    self.history_index = None;
                }
                Err(e) => self.fail(e),
            }
        }
    }

    fn fail(&mut self, error: Error) {
        self.buffer_mut().cursor = error.start.min(self.buffer().text.chars().count());
        self.buffer_mut().error = Some(error);
        self.graph_editing = self.graph_mode;
        self.fresh = false;
    }

    fn recall(&mut self, older: bool) {
        if self.history.is_empty() {
            return;
        }
        if older {
            if self.history_index.is_none() {
                self.history_draft = self.calculate.clone();
            }
            let index = self
                .history_index
                .unwrap_or(self.history.len())
                .saturating_sub(1);
            self.history_index = Some(index);
            self.calculate = Buffer::new(&self.history[index].expression);
        } else if let Some(index) = self.history_index {
            if index + 1 == self.history.len() {
                self.history_index = None;
                self.calculate = self.history_draft.clone();
            } else {
                self.history_index = Some(index + 1);
                self.calculate = Buffer::new(&self.history[index + 1].expression);
            }
        }
        self.fresh = false;
    }

    pub fn place_cursor(&mut self, index: usize) {
        self.opening_view = false;
        self.buffer_mut().cursor = index.min(self.buffer().text.chars().count());
        self.graph_editing = self.graph_mode;
        self.fresh = false;
        self.dirty = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn calculate_cartesian() -> Model {
        let mut m = Model::default();
        m.action("mode");
        m.action("calculate");
        m
    }

    #[test]
    fn opening_rosette_is_closed_and_fits_until_the_first_interaction() {
        let mut m = Model::default();
        assert!(m.graph_mode && m.polar && !m.graph_editing);
        assert_eq!(m.graph.text, DEFAULT_POLAR_SOURCE);
        let start = m.plotted.evaluate(0.).unwrap();
        let end = m.plotted.evaluate(std::f64::consts::TAU).unwrap();
        assert!((start - 4.).abs() < 1e-12 && (end - start).abs() < 1e-12);
        for (width, height) in [(240., 116.), (313., 240.), (940., 700.)] {
            m.fit_opening_view(width, height);
            assert!(8. * zoom_scale(m.zoom_level) <= width.min(height) * 0.95);
        }
        m.action("zoom-out");
        let chosen = m.zoom_level;
        m.fit_opening_view(240., 116.);
        assert_eq!(m.zoom_level, chosen);
        m.action("calculate");
        assert!(m.calculate.text.is_empty());
        m.action("7*6");
        m.action("enter");
        assert_eq!(m.answer, Some(42.));
        m.action("graph");
        assert_eq!(m.graph.text, DEFAULT_POLAR_SOURCE);
    }

    fn key(m: &mut Model, action: &str) {
        m.press_key(action, secondary_key(action).1);
    }

    #[test]
    fn second_is_one_shot_cancellable_and_preserves_normal_keys() {
        let mut m = calculate_cartesian();
        key(&mut m, "second");
        assert!(m.second);
        key(&mut m, "sin");
        assert_eq!(m.calculate.text, "asin(");
        assert!(!m.second);
        m.action("clear");
        key(&mut m, "second");
        key(&mut m, "second");
        assert!(!m.second);
        key(&mut m, "cos");
        assert_eq!(m.calculate.text, "cos(");
        m.action("clear");
        key(&mut m, "second");
        key(&mut m, "7");
        assert_eq!(m.calculate.text, "7");
        assert!(!m.second);
        key(&mut m, "second");
        m.action("escape");
        assert!(!m.second);
        key(&mut m, "second");
        m.action("s");
        assert_eq!(m.calculate.text, "7s");
        assert!(!m.second);
    }

    #[test]
    fn secondary_functions_evaluate_and_mode_keeps_its_drafts() {
        for (primary, suffix, result) in [
            ("sin", "1)", std::f64::consts::FRAC_PI_2),
            ("cos", "1)", 0.),
            ("tan", "1)", std::f64::consts::FRAC_PI_4),
            ("ln", "2)", std::f64::consts::E.powi(2)),
            ("log", "2)", 100.),
        ] {
            let mut m = calculate_cartesian();
            key(&mut m, "second");
            key(&mut m, primary);
            m.action(suffix);
            key(&mut m, "enter");
            assert!((m.answer.unwrap() - result).abs() < 1e-12, "{primary}");
        }
        let mut m = calculate_cartesian();
        m.action("4");
        m.action("enter");
        key(&mut m, "second");
        key(&mut m, "sqrt");
        m.action("enter");
        assert_eq!(m.answer, Some(16.));
        key(&mut m, "second");
        key(&mut m, "^");
        m.action("enter");
        assert_eq!(m.answer, Some(0.0625));
        key(&mut m, "second");
        key(&mut m, "variable");
        assert!(!m.polar && !m.second);
        key(&mut m, "graph");
        key(&mut m, "second");
        key(&mut m, "variable");
        assert!(m.polar && !m.second);
        key(&mut m, "clear");
        key(&mut m, "variable");
        assert_eq!(m.graph.text, "θ");
        key(&mut m, "graph");
        assert_eq!(m.plotted_source, "θ");
        assert!(m.graph_editing);
    }

    #[test]
    fn graph_button_submits_when_already_graphing_and_retains_errors() {
        let mut m = calculate_cartesian();
        m.action("7*6");
        m.action("graph");
        assert_eq!(m.calculate.text, "7*6");
        m.action("clear");
        m.action("cos(x)");
        m.action("graph");
        assert_eq!(m.plotted_source, "cos(x)");
        assert!(m.home_requested && m.graph_editing);
        m.action("+");
        m.action("graph");
        assert!(m.graph.error.is_some());
        assert_eq!(m.plotted_source, "cos(x)");
        assert_eq!(m.graph.text, "cos(x)+");
        assert!(m.graph_editing);
    }

    #[test]
    fn polar_mode_preserves_drafts_and_uses_the_active_variable() {
        let mut m = calculate_cartesian();
        m.action("mode");
        assert!(!m.polar);
        m.action("graph");
        m.action("clear");
        m.action("variable");
        assert_eq!(m.graph.text, "x");
        m.action("enter");
        m.action("+1");
        m.action("mode");
        assert!(m.polar && m.graph_editing && m.home_requested);
        assert_eq!(m.graph.text, DEFAULT_POLAR_SOURCE);
        m.action("clear");
        m.action("variable");
        assert_eq!(m.graph.text, "θ");
        m.action("enter");
        assert_eq!(m.plotted.evaluate(2.).unwrap(), 2.);
        m.action("+x");
        m.action("enter");
        assert!(m.graph.error.is_some());
        assert_eq!(m.plotted_source, "θ");
        m.action("mode");
        assert_eq!(m.graph.text, "x+1");
        assert_eq!(m.plotted_source, "x");
        m.action("mode");
        assert_eq!(m.graph.text, "θ+x");
        assert!(m.graph.error.is_some());
    }
    #[test]
    fn repeat_enter_uses_the_latest_answer() {
        let mut m = calculate_cartesian();
        m.action("enter");
        assert!(m.history.is_empty());
        m.action("7×6");
        for want in [42., 252., 1512.] {
            m.action("enter");
            assert_eq!(m.answer, Some(want));
        }
        assert_eq!(m.history[1].expression, "ans*(6)");
        m.action("ans*2+1");
        m.action("enter");
        assert_eq!(m.answer, Some(3025.));
        m.action("enter");
        assert_eq!(m.answer, Some(6051.));
        assert_eq!(m.history.last().unwrap().expression, "ans*2+1");
        m.action("graph");
        m.action("enter");
        m.action("calculate");
        m.action("enter");
        assert_eq!(m.answer, Some(12103.));
    }
    #[test]
    fn repeat_preserves_grouping_functions_and_errors() {
        for (source, first, second) in [
            ("2+(3+4)*5", 37., 72.),
            ("(2+3)×(4+5)", 45., 405.),
            ("2^3^2", 512., 512_f64.powi(9)),
            ("100/(2*5)", 10., 1.),
            ("sqrt(256)", 16., 4.),
            ("-3", -3., 3.),
            ("7", 7., 7.),
        ] {
            let mut m = calculate_cartesian();
            m.action(source);
            m.action("enter");
            assert_eq!(m.answer, Some(first), "{source}");
            m.action("enter");
            assert_eq!(m.answer, Some(second), "{source}: {}", m.calculate.text);
        }
        let mut m = calculate_cartesian();
        m.action("1e300*10");
        m.action("enter");
        for _ in 0..8 {
            m.action("enter");
        }
        assert!(m.calculate.error.is_some());
        assert_eq!(m.calculate.text, "ans*(10)");
        assert!(m.answer.unwrap().is_finite());
    }
    #[test]
    fn calculate_zoom_is_independent_and_history_reflows_without_loss() {
        let mut m = calculate_cartesian();
        for _ in 0..8 {
            m.action("zoom-in");
        }
        assert_eq!(m.text_zoom_level, MAX_TEXT_ZOOM);
        m.action("graph");
        m.action("zoom-out");
        assert_eq!(m.zoom_level, -1);
        m.action("calculate");
        assert_eq!(m.text_zoom_level, MAX_TEXT_ZOOM);
        for _ in 0..8 {
            m.action("zoom-out");
        }
        assert_eq!(m.text_zoom_level, MIN_TEXT_ZOOM);
        assert_eq!(m.zoom_level, -1);
        m.action("7×6");
        m.action("x");
        assert_eq!(m.calculate.text, "7×6");
        m.action("enter");
        for _ in 0..19 {
            m.action("enter");
        }
        let rows = m.history_rows(12);
        assert!(rows.len() > 50);
        assert!(rows.iter().all(|(s, _)| s.chars().count() <= 14));
        let rendered_answers: String = rows
            .iter()
            .filter(|(_, subdued)| !subdued)
            .map(|(s, _)| s.trim())
            .collect();
        let answers: String = m.history.iter().map(|e| e.result.as_str()).collect();
        assert_eq!(rendered_answers, answers);
    }
    #[test]
    fn graph_controls_preserve_drafts_and_zoom() {
        let mut m = calculate_cartesian();
        m.home_requested = false;
        m.action("zoom-in");
        assert_eq!(m.zoom_level, 0);
        m.action("graph");
        assert!(m.home_requested);
        m.action("clear");
        m.action("x^2");
        m.action("zoom-in");
        assert_eq!(m.zoom_level, 1);
        assert_eq!(m.plotted_source, "sin(x)");
        assert_eq!(m.graph.text, "x^2");
        m.action("graph");
        assert!(m.home_requested);
        assert_eq!(m.zoom_level, 1);
        assert!(m.graph_editing);
        m.action("left");
        assert_eq!(m.graph.cursor, 2);
        assert_eq!(m.pan, (0., 0.));
        for _ in 0..20 {
            m.action("zoom-in");
        }
        assert_eq!(m.zoom_level, MAX_ZOOM);
        for _ in 0..20 {
            m.action("zoom-out");
        }
        assert_eq!(m.zoom_level, MIN_ZOOM);
        m.action("calculate");
        m.action("graph");
        assert_eq!(m.zoom_level, MIN_ZOOM);
        assert_eq!(m.graph.text, "x^2");
    }
    #[test]
    fn power_shortcuts_continue_the_last_answer() {
        let mut m = calculate_cartesian();
        m.action("4");
        m.action("square");
        m.action("enter");
        assert_eq!(m.answer, Some(16.));
        m.action("reciprocal");
        m.action("enter");
        assert_eq!(m.answer, Some(0.0625));
        m.action("square");
        assert_eq!(m.calculate.text, "ans^2");
    }
    #[test]
    fn unicode_edit_and_error_recovery() {
        let mut m = calculate_cartesian();
        m.action("π");
        m.action("+");
        m.action("sqrt");
        m.action("-");
        m.action("1");
        m.action(")");
        m.action("enter");
        assert!(m.calculate.error.is_some());
        assert_eq!(m.calculate.text, "π+sqrt(-1)");
        m.action("clear");
        m.action("π");
        m.action("backspace");
        assert_eq!(m.calculate.text, "");
        m.action("2");
        m.action("enter");
        assert_eq!(m.answer, Some(2.));
        m.action("+");
        m.action("3");
        m.action("enter");
        assert_eq!(m.answer, Some(5.));
    }
    #[test]
    fn history_and_mode_buffers_survive() {
        let mut m = calculate_cartesian();
        m.action("25");
        m.action("enter");
        m.action("7");
        m.action("up");
        assert_eq!(m.calculate.text, "25");
        m.action("down");
        assert_eq!(m.calculate.text, "7");
        m.action("graph");
        m.action("clear");
        m.action("x^2");
        m.action("enter");
        m.action("calculate");
        assert_eq!(m.calculate.text, "7");
        m.action("graph");
        assert_eq!(m.graph.text, "x^2");
        m.action("clear");
        m.action("(");
        m.action("enter");
        assert_eq!(m.plotted_source, "x^2");
        assert!(m.graph.error.is_some());
    }
    #[test]
    fn graph_answer_is_frozen() {
        let mut m = calculate_cartesian();
        m.action("2");
        m.action("enter");
        m.action("graph");
        m.action("clear");
        m.action("ans*x");
        m.action("enter");
        m.action("calculate");
        m.action("9");
        m.action("enter");
        assert_eq!(m.plotted.evaluate(3.).unwrap(), 6.);
    }
    #[test]
    fn cursor_limits_history_cap_and_graph_focus() {
        let mut m = calculate_cartesian();
        m.action("12π");
        m.place_cursor(1);
        m.action("delete");
        assert_eq!(m.calculate.text, "1π");
        m.place_cursor(999);
        assert_eq!(m.calculate.cursor, 2);
        m.action("clear");
        m.action(&"1".repeat(MAX_INPUT));
        m.action("2");
        assert_eq!(m.calculate.text.len(), MAX_INPUT);
        assert!(m.calculate.error.is_some());
        m.action("backspace");
        assert!(m.calculate.error.is_none());
        for i in 0..25 {
            m.action("clear");
            m.action(&i.to_string());
            m.action("enter");
        }
        assert_eq!(m.history.len(), 20);
        assert_eq!(m.history[0].result, "5");
        m.action("graph");
        m.home_requested = false;
        assert!(m.graph_editing);
        m.action("escape");
        m.action("right");
        assert_eq!(m.pan, (48., 0.));
        m.action("home");
        assert!(m.home_requested);
        m.place_cursor(3);
        m.action("home");
        assert_eq!(m.graph.cursor, 0);
    }
}
