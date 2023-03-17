use std::cmp::Ordering;
use std::ops::{Add, Div, Mul, Neg, Rem, Sub};
use std::rc::Rc;

/// Numeric is a module that wraps numeric literals in Pax
/// It encapsulates the built-in Rust numeric scalar types and defines behavior across them
#[derive(Clone, Copy, Debug)]
pub enum Numeric {
    Integer(isize),
    Float(f64),
}

impl From<u8> for Numeric {
    fn from(value: u8) -> Self {
        Numeric::Integer(value as isize)
    }
}
impl From<u16> for Numeric {
    fn from(value: u16) -> Self {
        Numeric::Integer(value as isize)
    }
}
impl From<u32> for Numeric {
    fn from(value: u32) -> Self {
        Numeric::Integer(value as isize)
    }
}
impl From<u64> for Numeric {
    fn from(value: u64) -> Self {
        Numeric::Integer(value as isize)
    }
}
impl From<u128> for Numeric {
    fn from(value: u128) -> Self {
        Numeric::Integer(value as isize)
    }
}
impl From<usize> for Numeric {
    fn from(value: usize) -> Self {
        Numeric::Integer(value as isize)
    }
}
impl From<i8> for Numeric {
    fn from(value: i8) -> Self {
        Numeric::Integer(value as isize)
    }
}
impl From<i16> for Numeric {
    fn from(value: i16) -> Self {
        Numeric::Integer(value as isize)
    }
}
impl From<i32> for Numeric {
    fn from(value: i32) -> Self {
        Numeric::Integer(value as isize)
    }
}
impl From<i64> for Numeric {
    fn from(value: i64) -> Self {
        Numeric::Integer(value as isize)
    }
}
impl From<i128> for Numeric {
    fn from(value: i128) -> Self {
        Numeric::Integer(value as isize)
    }
}
impl From<isize> for Numeric {
    fn from(value: isize) -> Self {
        Numeric::Integer(value)
    }
}
impl From<f64> for Numeric {
    fn from(value: f64) -> Self {
        Numeric::Float(value)
    }
}

impl Numeric {
    fn widen(
        f: fn(f64, f64) -> f64,
        i: fn(isize, isize) -> isize,
        lhs: Numeric,
        rhs: Numeric,
    ) -> Numeric {
        match lhs {
            Numeric::Integer(x) => match rhs {
                Numeric::Integer(y) => Numeric::Integer(i(x, y)),
                Numeric::Float(y) => Numeric::Float(f(x as f64, y)),
            },
            Numeric::Float(x) => match rhs {
                Numeric::Integer(y) => Numeric::Float(f(x, y as f64)),
                Numeric::Float(y) => Numeric::Float(f(x, y)),
            },
        }
    }

    pub fn get_as_float(self) -> f64 {
        match self {
            Numeric::Integer(value) => value as f64,
            Numeric::Float(value) => value,
        }
    }

    pub fn pow(x: Numeric, exp: Numeric) -> Numeric {
        Numeric::widen(|x, y| x.powf(y), |x, y| x.pow(y as u32), x, exp)
    }

    fn float_eq(x: f64, y: f64, epsilon: f64) -> bool {
        if x.is_nan() || y.is_nan() {
            return false;
        }

        (x - y).abs() <= epsilon
    }
}

impl Add for Numeric {
    type Output = Numeric;

    fn add(self, rhs: Self) -> Self::Output {
        Numeric::widen(|x, y| x + y, |x, y| x + y, self, rhs)
    }
}

impl Sub for Numeric {
    type Output = Numeric;

    fn sub(self, rhs: Self) -> Self::Output {
        Numeric::widen(|x, y| x - y, |x, y| x - y, self, rhs)
    }
}

impl Mul for Numeric {
    type Output = Numeric;

    fn mul(self, rhs: Self) -> Self::Output {
        Numeric::widen(|x, y| x * y, |x, y| x * y, self, rhs)
    }
}

impl Div for Numeric {
    type Output = Numeric;

    fn div(self, rhs: Self) -> Self::Output {
        Numeric::widen(|x, y| x / y, |x, y| x / y, self, rhs)
    }
}

impl Neg for Numeric {
    type Output = Numeric;

    fn neg(self) -> Self::Output {
        match self {
            Numeric::Integer(value) => Numeric::Integer(-value),
            Numeric::Float(value) => Numeric::Float(-value),
        }
    }
}

impl Rem for Numeric {
    type Output = Numeric;

    fn rem(self, rhs: Self) -> Self::Output {
        Numeric::widen(|x, y| x % y, |x, y| x % y, self, rhs)
    }
}

impl PartialEq<Self> for Numeric {
    fn eq(&self, other: &Self) -> bool {
        match *self {
            Numeric::Integer(x) => match *other {
                Numeric::Integer(y) => x == y,
                Numeric::Float(y) => Numeric::float_eq(x as f64, y, f64::EPSILON),
            },
            Numeric::Float(x) => match *other {
                Numeric::Integer(y) => Numeric::float_eq(x, y as f64, f64::EPSILON),
                Numeric::Float(y) => Numeric::float_eq(x, y, f64::EPSILON),
            },
        }
    }
}

impl PartialOrd<Self> for Numeric {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match *self {
            Numeric::Integer(x) => match *other {
                Numeric::Integer(y) => x.partial_cmp(&y),
                Numeric::Float(y) => (x as f64).partial_cmp(&y),
            },
            Numeric::Float(x) => match *other {
                Numeric::Integer(y) => x.partial_cmp(&(y as f64)),
                Numeric::Float(y) => x.partial_cmp(&y),
            },
        }
    }
}

/// Helper functions to ease working with built-ins

impl Mul<Numeric> for f64 {
    type Output = f64;

    fn mul(self, rhs: Numeric) -> Self::Output {
        self * rhs.get_as_float()
    }
}

impl Div<Numeric> for f64 {
    type Output = f64;

    fn div(self, rhs: Numeric) -> Self::Output {
        self / rhs.get_as_float()
    }
}

impl Div<f64> for Numeric {
    type Output = f64;

    fn div(self, rhs: f64) -> Self::Output {
        self.get_as_float() / rhs
    }
}

/// Tests for Numeric

#[cfg(test)]
mod tests {
    use crate::numeric::Numeric;

    #[test]
    fn test_widen() {
        let integer_a = Numeric::from(1 as usize);
        let integer_b = Numeric::from(2 as usize);
        let float_a = Numeric::from(3.0);
        let float_b = Numeric::from(4.0);

        assert_eq!(integer_a+integer_b, Numeric::Integer(3 as isize));
        assert_eq!(integer_a+float_a, Numeric::Float(4.0));
        assert_eq!(float_a+float_b, Numeric::Float(7.0));
    }

    #[test]
    fn test_cmp() {
        let integer_a = Numeric::from(1 as usize);
        let integer_b = Numeric::from(2 as usize);
        let float_a = Numeric::from(3.0);
        let float_b = Numeric::from(4.0);
        let float_c = Numeric::from(3.0000000001);

        assert_eq!(integer_a < integer_b, true);
        assert_eq!(integer_a > float_a, false);
        assert_eq!(float_a <= float_b, true);
        assert_eq!(float_b >= float_a, true);
        assert_eq!(float_c > float_a, true)
    }

    #[test]
    fn test_eq() {
        let integer_a = Numeric::from(1 as usize);
        let integer_b = Numeric::from(2 as usize);
        let integer_c = Numeric::from(3 as usize);
        let float_a = Numeric::from(3.0);
        let float_b = Numeric::from(4.0);
        let float_c = Numeric::from(3.0000000001);

        assert_eq!(integer_a != integer_b, true);
        assert_eq!(integer_b == integer_b, true);
        assert_eq!(integer_c == float_a, true);
        assert_eq!(integer_c != float_b, true);
        assert_eq!(float_a != float_c, true);
    }

    #[test]
    fn test_get_as_float() {
        let a = Numeric::from(3.0000000001);
        let b = 3.0000000001;

        assert_eq!(a.get_as_float(), b);
    }

    #[test]
    fn test_float_eq() {
        let a = 100.01;
        let b = 100.011;
        let epsilon_a = 0.1;
        let epsilon_b = 0.001;
        let epsilon_c = 0.0001;

        assert_eq!(Numeric::float_eq(a,b, epsilon_a ), true);
        assert_eq!(Numeric::float_eq(a,b, epsilon_b ), true);
        assert_eq!(Numeric::float_eq(a,b, epsilon_c ), false);
    }
}
