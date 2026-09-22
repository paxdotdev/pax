//! The calculator's bounded, real-valued expression language, shared by both modes.

use std::f64::consts::{E, FRAC_PI_2, PI, TAU};

pub const MAX_INPUT: usize = 256;
const MAX_DEPTH: usize = 32;
const MAX_NODES: usize = 256;

#[derive(Clone, Debug, PartialEq)]
pub struct Error {
    pub message: String,
    pub start: usize,
    pub end: usize,
}

impl Error {
    fn at(message: &str, start: usize, end: usize) -> Self {
        Self {
            message: message.into(),
            start,
            end: end.max(start + 1),
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum Function {
    Sin,
    Cos,
    Tan,
    Sec,
    Csc,
    Atan,
    Sqrt,
    Ln,
    Log,
}

#[derive(Clone, Debug)]
enum Kind {
    Number(f64),
    X,
    Neg(usize),
    Call(Function, usize),
    Binary(char, usize, usize),
}

#[derive(Clone, Debug)]
struct Node {
    kind: Kind,
    start: usize,
    end: usize,
    variable: bool,
}

#[derive(Clone, Debug)]
pub struct Expression {
    nodes: Vec<Node>,
    root: usize,
    uses_answer: bool,
}

#[derive(Clone, Debug, PartialEq)]
enum Token {
    Number(f64),
    Name(String),
    Symbol(char),
    End,
}

#[derive(Clone, Debug)]
struct Lexeme {
    token: Token,
    start: usize,
    end: usize,
}

impl Expression {
    pub fn parse(source: &str, graph: bool, answer: Option<f64>) -> Result<Self, Error> {
        let chars: Vec<_> = source.chars().collect();
        if chars.len() > MAX_INPUT {
            return Err(Error::at(
                "Expression is too long (256 characters)",
                MAX_INPUT,
                chars.len(),
            ));
        }
        let mut tokens = Vec::new();
        let mut p = 0;
        while p < chars.len() {
            if chars[p].is_whitespace() {
                p += 1;
                continue;
            }
            let start = p;
            let c = chars[p];
            let token = if c.is_ascii_digit() || c == '.' {
                let mut digits = 0;
                while p < chars.len() && chars[p].is_ascii_digit() {
                    p += 1;
                    digits += 1;
                }
                if chars.get(p) == Some(&'.') {
                    p += 1;
                    while p < chars.len() && chars[p].is_ascii_digit() {
                        p += 1;
                        digits += 1;
                    }
                }
                if digits == 0 {
                    return Err(Error::at("Expected a number", start, p));
                }
                if matches!(chars.get(p), Some('e' | 'E')) {
                    p += 1;
                    if matches!(chars.get(p), Some('+' | '-')) {
                        p += 1;
                    }
                    let exponent = p;
                    while p < chars.len() && chars[p].is_ascii_digit() {
                        p += 1;
                    }
                    if p == exponent {
                        return Err(Error::at("Expected exponent digits", start, p));
                    }
                }
                let spelling: String = chars[start..p].iter().collect();
                let n = spelling
                    .parse::<f64>()
                    .map_err(|_| Error::at("Invalid number", start, p))?;
                if !n.is_finite() {
                    return Err(Error::at("Number is too large", start, p));
                }
                Token::Number(n)
            } else if c.is_ascii_alphabetic() {
                p += 1;
                while p < chars.len() && chars[p].is_ascii_alphanumeric() {
                    p += 1;
                }
                Token::Name(
                    chars[start..p]
                        .iter()
                        .collect::<String>()
                        .to_ascii_lowercase(),
                )
            } else {
                p += 1;
                match c {
                    'π' => Token::Name("pi".into()),
                    '√' => Token::Name("sqrt".into()),
                    '×' => Token::Symbol('*'),
                    '÷' => Token::Symbol('/'),
                    '−' => Token::Symbol('-'),
                    '+' | '-' | '*' | '/' | '^' | '(' | ')' => Token::Symbol(c),
                    _ => return Err(Error::at("Unrecognized character", start, p)),
                }
            };
            tokens.push(Lexeme {
                token,
                start,
                end: p,
            });
            if tokens.len() > MAX_NODES {
                return Err(Error::at("Too many terms", start, p));
            }
        }
        tokens.push(Lexeme {
            token: Token::End,
            start: p,
            end: p,
        });
        let uses_answer = tokens.iter().any(|t| t.token == Token::Name("ans".into()));
        let mut parser = Parser {
            tokens,
            at: 0,
            nodes: Vec::new(),
            graph,
            answer,
        };
        let root = parser.expression(0, 0)?;
        let tail = &parser.tokens[parser.at];
        if tail.token != Token::End {
            return Err(Error::at(
                "Expected an operator (use × to multiply)",
                tail.start,
                tail.end,
            ));
        }
        Ok(Self {
            nodes: parser.nodes,
            root,
            uses_answer,
        })
    }

    /// Repeat the outermost operation with the latest answer; explicit `ans` wins.
    pub fn repeat_source(&self, source: &str) -> String {
        if self.uses_answer {
            return source.into();
        }
        match self.nodes[self.root].kind {
            Kind::Binary(op, _, right) => {
                let n = &self.nodes[right];
                let rhs: String = source.chars().skip(n.start).take(n.end - n.start).collect();
                format!("ans{op}({rhs})")
            }
            Kind::Call(f, _) => format!(
                "{}(ans)",
                match f {
                    Function::Sin => "sin",
                    Function::Cos => "cos",
                    Function::Tan => "tan",
                    Function::Sec => "sec",
                    Function::Csc => "csc",
                    Function::Atan => "atan",
                    Function::Sqrt => "sqrt",
                    Function::Ln => "ln",
                    Function::Log => "log",
                }
            ),
            Kind::Neg(_) => "-ans".into(),
            _ => "ans".into(),
        }
    }

    pub fn evaluate(&self, x: f64) -> Result<f64, Error> {
        self.eval(self.root, x, &mut 1024)
    }

    pub fn sample(&self, x: f64, budget: &mut usize) -> Option<f64> {
        self.eval(self.root, x, budget).ok()
    }

    fn eval(&self, index: usize, x: f64, budget: &mut usize) -> Result<f64, Error> {
        let n = &self.nodes[index];
        if *budget == 0 {
            return Err(Error::at("Calculation limit reached", n.start, n.end));
        }
        *budget -= 1;
        let fail = |message: &str| Error::at(message, n.start, n.end);
        let result = match n.kind {
            Kind::Number(v) => v,
            Kind::X => x,
            Kind::Neg(a) => -self.eval(a, x, budget)?,
            Kind::Binary(op, a, b) => {
                let a = self.eval(a, x, budget)?;
                let b = self.eval(b, x, budget)?;
                match op {
                    '+' => a + b,
                    '-' => a - b,
                    '*' => a * b,
                    '/' if b == 0.0 => return Err(fail("Cannot divide by zero")),
                    '/' => a / b,
                    '^' if a == 0.0 && b <= 0.0 => {
                        return Err(fail("Zero needs a positive exponent"))
                    }
                    '^' if a < 0.0 && b.fract() != 0.0 => {
                        return Err(fail("Negative base needs an integer power"))
                    }
                    '^' => a.powf(b),
                    _ => unreachable!(),
                }
            }
            Kind::Call(f, a) => {
                let a = self.eval(a, x, budget)?;
                match f {
                    Function::Sin => a.sin(),
                    Function::Cos => a.cos(),
                    Function::Tan if a.cos().abs() <= 4.0 * f64::EPSILON => {
                        return Err(fail("Tangent is undefined here"))
                    }
                    Function::Tan => a.tan(),
                    Function::Sec if a.cos().abs() <= 4.0 * f64::EPSILON => {
                        return Err(fail("Secant is undefined here"))
                    }
                    Function::Csc if a.sin().abs() <= 4.0 * f64::EPSILON => {
                        return Err(fail("Cosecant is undefined here"))
                    }
                    Function::Sec => 1.0 / a.cos(),
                    Function::Csc => 1.0 / a.sin(),
                    Function::Atan => a.atan(),
                    Function::Sqrt if a < 0.0 => {
                        return Err(fail("sqrt needs a nonnegative value"))
                    }
                    Function::Sqrt => a.sqrt(),
                    Function::Ln | Function::Log if a <= 0.0 => {
                        return Err(fail("log needs a positive value"))
                    }
                    Function::Ln => a.ln(),
                    Function::Log => a.log10(),
                }
            }
        };
        if result.is_finite() {
            Ok(result)
        } else {
            Err(fail("Result is outside the numeric range"))
        }
    }

    /// A conservative domain and range enclosure. `None` requires subdivision or a gap.
    pub fn range(&self, x: Range, budget: &mut usize) -> Option<Range> {
        self.enclose(self.root, x, budget)
    }

    fn enclose(&self, i: usize, x: Range, budget: &mut usize) -> Option<Range> {
        if *budget == 0 {
            return None;
        }
        *budget -= 1;
        let node = &self.nodes[i];
        if !node.variable {
            return self.eval(i, 0.0, budget).ok().map(Range::point);
        }
        match node.kind {
            Kind::Number(v) => Some(Range::point(v)),
            Kind::X => Some(x),
            Kind::Neg(a) => {
                let a = self.enclose(a, x, budget)?;
                Range::new(-a.hi, -a.lo)
            }
            Kind::Binary(op, a, b) => {
                let a = self.enclose(a, x, budget)?;
                let b = self.enclose(b, x, budget)?;
                match op {
                    '+' => Range::new(a.lo + b.lo, a.hi + b.hi),
                    '-' => Range::new(a.lo - b.hi, a.hi - b.lo),
                    '*' => a.multiply(b),
                    '/' if b.lo <= 0.0 && b.hi >= 0.0 => None,
                    '/' => a.multiply(Range::new(1.0 / b.hi, 1.0 / b.lo)?),
                    '^' if b.lo == b.hi && b.lo.fract() == 0.0 && b.lo.abs() <= 1024.0 => {
                        let p = b.lo;
                        if a.lo <= 0.0 && a.hi >= 0.0 && p <= 0.0 {
                            return None;
                        }
                        let left = a.lo.powf(p);
                        let right = a.hi.powf(p);
                        let low = if p > 0.0 && p % 2.0 == 0.0 && a.lo <= 0.0 && a.hi >= 0.0 {
                            0.0
                        } else {
                            left.min(right)
                        };
                        Range::new(low, left.max(right))
                    }
                    '^' if a.lo > 0.0 => {
                        let log = Range::new(a.lo.ln(), a.hi.ln())?.multiply(b)?;
                        Range::new(log.lo.exp(), log.hi.exp())
                    }
                    '^' => None,
                    _ => unreachable!(),
                }
            }
            Kind::Call(f, a) => {
                let a = self.enclose(a, x, budget)?;
                match f {
                    Function::Sin => a.sin(),
                    Function::Cos => Range::new(a.lo + FRAC_PI_2, a.hi + FRAC_PI_2)?.sin(),
                    Function::Sec => Range::new(a.lo + FRAC_PI_2, a.hi + FRAC_PI_2)?
                        .sin()?
                        .reciprocal(),
                    Function::Csc => a.sin()?.reciprocal(),
                    Function::Atan => Range::new(a.lo.atan(), a.hi.atan()),
                    Function::Tan => {
                        if a.lo.abs().max(a.hi.abs()) > 1e12
                            || ((a.lo + FRAC_PI_2) / PI).floor()
                                != ((a.hi + FRAC_PI_2) / PI).floor()
                        {
                            return None;
                        }
                        Range::new(a.lo.tan(), a.hi.tan())
                    }
                    Function::Sqrt if a.lo >= 0.0 => Range::new(a.lo.sqrt(), a.hi.sqrt()),
                    Function::Ln if a.lo > 0.0 => Range::new(a.lo.ln(), a.hi.ln()),
                    Function::Log if a.lo > 0.0 => Range::new(a.lo.log10(), a.hi.log10()),
                    _ => None,
                }
            }
        }
    }
}

struct Parser {
    tokens: Vec<Lexeme>,
    at: usize,
    nodes: Vec<Node>,
    graph: bool,
    answer: Option<f64>,
}

impl Parser {
    fn push(&mut self, kind: Kind, start: usize, end: usize) -> Result<usize, Error> {
        if self.nodes.len() >= MAX_NODES {
            return Err(Error::at("Too many terms", start, end));
        }
        let variable = match kind {
            Kind::X => true,
            Kind::Neg(a) | Kind::Call(_, a) => self.nodes[a].variable,
            Kind::Binary(_, a, b) => self.nodes[a].variable || self.nodes[b].variable,
            _ => false,
        };
        self.nodes.push(Node {
            kind,
            start,
            end,
            variable,
        });
        Ok(self.nodes.len() - 1)
    }

    fn expression(&mut self, min: u8, depth: usize) -> Result<usize, Error> {
        let t = self.tokens[self.at].clone();
        if depth >= MAX_DEPTH {
            return Err(Error::at("Expression is nested too deeply", t.start, t.end));
        }
        self.at += 1;
        let mut left = match t.token {
            Token::Number(v) => self.push(Kind::Number(v), t.start, t.end)?,
            Token::Name(ref name) => {
                let number = match name.as_str() {
                    "pi" => Some(PI),
                    "e" => Some(E),
                    "ans" => Some(
                        self.answer
                            .ok_or_else(|| Error::at("No previous answer yet", t.start, t.end))?,
                    ),
                    _ => None,
                };
                if let Some(v) = number {
                    self.push(Kind::Number(v), t.start, t.end)?
                } else if name == "x" {
                    if !self.graph {
                        return Err(Error::at("Use x in Graph mode", t.start, t.end));
                    }
                    self.push(Kind::X, t.start, t.end)?
                } else {
                    let f = match name.as_str() {
                        "sin" => Function::Sin,
                        "cos" => Function::Cos,
                        "tan" => Function::Tan,
                        "sec" => Function::Sec,
                        "csc" => Function::Csc,
                        "atan" => Function::Atan,
                        "sqrt" => Function::Sqrt,
                        "ln" => Function::Ln,
                        "log" | "log10" => Function::Log,
                        _ => return Err(Error::at("Unknown function or constant", t.start, t.end)),
                    };
                    if self.tokens[self.at].token != Token::Symbol('(') {
                        return Err(Error::at("Function needs parentheses", t.start, t.end));
                    }
                    self.at += 1;
                    let a = self.expression(0, depth + 1)?;
                    let end = self.close()?;
                    self.push(Kind::Call(f, a), t.start, end)?
                }
            }
            Token::Symbol('(') => {
                let a = self.expression(0, depth + 1)?;
                let end = self.close()?;
                // Keep grouping in source spans so repeat-Enter retains the exact RHS.
                self.nodes[a].start = t.start;
                self.nodes[a].end = end;
                a
            }
            Token::Symbol(sign @ ('-' | '+')) => {
                let a = self.expression(25, depth + 1)?;
                if sign == '-' {
                    self.push(Kind::Neg(a), t.start, self.nodes[a].end)?
                } else {
                    a
                }
            }
            _ => return Err(Error::at("Expected a number or expression", t.start, t.end)),
        };
        loop {
            let op = match self.tokens[self.at].token {
                Token::Symbol(c @ ('+' | '-' | '*' | '/' | '^')) => c,
                _ => break,
            };
            let binding = match op {
                '+' | '-' => 10,
                '*' | '/' => 20,
                '^' => 30,
                _ => unreachable!(),
            };
            if binding < min {
                break;
            }
            self.at += 1;
            let right =
                self.expression(if op == '^' { binding } else { binding + 1 }, depth + 1)?;
            left = self.push(
                Kind::Binary(op, left, right),
                self.nodes[left].start,
                self.nodes[right].end,
            )?;
        }
        Ok(left)
    }

    fn close(&mut self) -> Result<usize, Error> {
        let t = &self.tokens[self.at];
        if t.token != Token::Symbol(')') {
            return Err(Error::at("Expected ')'", t.start, t.end));
        }
        self.at += 1;
        Ok(t.end)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Range {
    pub lo: f64,
    pub hi: f64,
}

impl Range {
    fn reciprocal(self) -> Option<Self> {
        if self.lo <= 0. && self.hi >= 0. {
            None
        } else {
            Self::new(1. / self.hi, 1. / self.lo)
        }
    }
    pub fn point(v: f64) -> Self {
        Self { lo: v, hi: v }
    }
    pub fn new(lo: f64, hi: f64) -> Option<Self> {
        if !lo.is_finite() || !hi.is_finite() || lo > hi {
            return None;
        }
        // Widen computed bounds to cover floating-point roundoff in the enclosure.
        let pad = lo.abs().max(hi.abs()).max(f64::MIN_POSITIVE) * 8.0 * f64::EPSILON;
        let lo = lo - pad;
        let hi = hi + pad;
        (lo.is_finite() && hi.is_finite()).then_some(Self { lo, hi })
    }
    fn multiply(self, b: Self) -> Option<Self> {
        let v = [
            self.lo * b.lo,
            self.lo * b.hi,
            self.hi * b.lo,
            self.hi * b.hi,
        ];
        if v.iter().any(|v| !v.is_finite()) {
            return None;
        }
        Self::new(
            v.iter().copied().fold(f64::INFINITY, f64::min),
            v.iter().copied().fold(f64::NEG_INFINITY, f64::max),
        )
    }
    fn sin(self) -> Option<Self> {
        if self.lo.abs().max(self.hi.abs()) > 1e12 {
            return None;
        }
        if self.hi - self.lo >= TAU {
            return Some(Self { lo: -1.0, hi: 1.0 });
        }
        let mut lo = self.lo.sin().min(self.hi.sin());
        let mut hi = self.lo.sin().max(self.hi.sin());
        if ((self.lo - FRAC_PI_2) / TAU).ceil() <= ((self.hi - FRAC_PI_2) / TAU).floor() {
            hi = 1.0;
        }
        if ((self.lo + FRAC_PI_2) / TAU).ceil() <= ((self.hi + FRAC_PI_2) / TAU).floor() {
            lo = -1.0;
        }
        Self::new(lo, hi)
    }
}

pub fn format_number(n: f64) -> String {
    if n == 0.0 {
        return "0".into();
    }
    let exponent = n.abs().log10().floor() as i32;
    if !(-6..12).contains(&exponent) {
        let raw = format!("{n:.11e}");
        let (mantissa, exp) = raw.split_once('e').unwrap();
        format!(
            "{}e{}",
            mantissa.trim_end_matches('0').trim_end_matches('.'),
            exp
        )
    } else {
        let places = (11 - exponent).max(0) as usize;
        let raw = format!("{n:.places$}");
        if raw.contains('.') {
            raw.trim_end_matches('0').trim_end_matches('.').into()
        } else {
            raw
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn value(s: &str) -> f64 {
        Expression::parse(s, false, None)
            .unwrap()
            .evaluate(0.0)
            .unwrap()
    }
    #[test]
    fn precedence_and_functions() {
        for (s, want) in [
            ("2^3^2", 512.),
            ("-2^2", -4.),
            ("(-2)^2", 4.),
            ("2^-3", 0.125),
            ("(2+3)*4", 20.),
            ("sqrt(ln(e)^2)", 1.),
            ("log(100)", 2.),
            ("1e-3 + .5", 0.501),
            ("2×3−1", 5.),
            ("sec(0)", 1.),
            ("csc(pi/2)", 1.),
            ("atan(1)", PI / 4.),
        ] {
            assert!((value(s) - want).abs() < 1e-10, "{s}");
        }
        assert!(value("sin(π)^2") < 1e-30);
    }
    #[test]
    fn errors_are_explicit() {
        for s in [
            "",
            "2pi",
            "1e",
            "1..2",
            "sin 3",
            "sqrt(",
            "(1+2",
            "2)",
            "unknown(2)",
            "x",
            "ans",
            ".",
        ] {
            assert!(Expression::parse(s, false, None).is_err(), "{s}");
        }
        for s in [
            "1/0",
            "sqrt(-1)",
            "ln(0)",
            "(-2)^.5",
            "0^0",
            "0^-1",
            "10^999",
            "tan(pi/2)",
            "sec(pi/2)",
            "csc(0)",
            "csc(pi)",
        ] {
            assert!(
                Expression::parse(s, false, None)
                    .unwrap()
                    .evaluate(0.)
                    .is_err(),
                "{s}"
            );
        }
    }
    #[test]
    fn limits_and_spans() {
        assert!(Expression::parse(&"1".repeat(257), false, None).is_err());
        assert!(Expression::parse(
            &format!("{}1{}", "(".repeat(40), ")".repeat(40)),
            false,
            None
        )
        .is_err());
        let err = Expression::parse("π + ?", false, None).unwrap_err();
        assert_eq!(err.start, 4);
        assert_eq!(
            Expression::parse("ans+1", false, Some(4.))
                .unwrap()
                .evaluate(0.)
                .unwrap(),
            5.
        );
    }
    #[test]
    fn domains_are_not_bridged() {
        for s in [
            "1/x",
            "1/(x-.12345)",
            "tan(x)",
            "sec(x)",
            "csc(x)",
            "sqrt(x)",
            "ln(x)",
            "x^-2",
        ] {
            assert!(
                Expression::parse(s, true, None)
                    .unwrap()
                    .range(Range { lo: -2., hi: 2. }, &mut 1000)
                    .is_none(),
                "{s}"
            );
        }
        let square = Expression::parse("x^2", true, None)
            .unwrap()
            .range(Range { lo: -2., hi: 2. }, &mut 1000)
            .unwrap();
        assert!(square.lo <= 0. && square.hi >= 4.);
    }
    #[test]
    fn format_preserves_small_values() {
        assert_eq!(format_number(-0.), "0");
        assert_eq!(format_number(25.), "25");
        assert_eq!(format_number(1e-20), "1e-20");
        assert_eq!(format_number(PI), "3.14159265359");
    }
    #[test]
    fn accepted_enclosures_cover_evaluated_values() {
        for source in [
            "sin(x)",
            "cos(x)",
            "tan(x)",
            "sec(x)",
            "csc(x)",
            "atan(x)",
            "1/(x-.12345)",
            "x^2",
            "x^3",
            "x^-2",
            "2^x",
            "x^.5",
            "sqrt(x)",
            "ln(x)",
            "log(x)",
            "sin(x)*cos(x)",
            "(x-2)/(x+3)",
        ] {
            let e = Expression::parse(source, true, None).unwrap();
            for i in -100..100 {
                let lo = i as f64 * 0.1;
                let hi = lo + 0.13;
                if let Some(r) = e.range(Range { lo, hi }, &mut 10000) {
                    for j in 0..=20 {
                        let x = lo + (hi - lo) * j as f64 / 20.;
                        let y = e.evaluate(x).expect(source);
                        assert!(
                            r.lo <= y && y <= r.hi,
                            "{source} at {x}: {r:?} excludes {y}"
                        );
                    }
                }
            }
        }
        assert!(Range::new(f64::MAX, f64::MAX).is_none());
    }
}
