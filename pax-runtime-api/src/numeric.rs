use serde::{Deserialize, Serialize};

use crate::{Interpolatable, IntoableLiteral, Size};
use std::cmp::Ordering;
use std::convert::TryFrom;
use std::ops::{Add, Div, Mul, Neg, Rem, Sub};

/// Numeric is a module that wraps numeric literals in Pax
/// It encapsulates the built-in Rust numeric scalar types and defines behavior across them
#[cfg_attr(debug_assertions, derive(Debug))]
#[derive(Clone, Copy, Serialize, Deserialize)]
#[serde(crate = "crate::serde")]
pub enum Numeric {
    Integer(isize),
    Float(f64),
}

impl From<IntoableLiteral> for Numeric {
    fn from(value: IntoableLiteral) -> Self {
        match value {
            IntoableLiteral::Numeric(n) => n,
            _ => {
                unreachable!()
            }
        }
    }
}

impl From<Numeric> for Size {
    fn from(value: Numeric) -> Self {
        Size::Pixels(value)
    }
}

impl Default for Numeric {
    fn default() -> Self {
        Self::Integer(0)
    }
}

impl Interpolatable for Numeric {
    fn interpolate(&self, other: &Self, t: f64) -> Self {
        Self::Float(self.to_float() + ((other.to_float() - self.to_float()) * t))
    }
}

impl PartialEq<f64> for Numeric {
    fn eq(&self, other: &f64) -> bool {
        match self {
            Numeric::Float(f) => f == other,
            _ => false,
        }
    }
}

impl PartialEq<isize> for Numeric {
    fn eq(&self, other: &isize) -> bool {
        match self {
            Numeric::Integer(i) => i == other,
            _ => false,
        }
    }
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

impl From<&u8> for Numeric {
    fn from(value: &u8) -> Self {
        Numeric::Integer(*value as isize)
    }
}
impl From<&u16> for Numeric {
    fn from(value: &u16) -> Self {
        Numeric::Integer(*value as isize)
    }
}
impl From<&u32> for Numeric {
    fn from(value: &u32) -> Self {
        Numeric::Integer(*value as isize)
    }
}
impl From<&u64> for Numeric {
    fn from(value: &u64) -> Self {
        Numeric::Integer(*value as isize)
    }
}
impl From<&u128> for Numeric {
    fn from(value: &u128) -> Self {
        Numeric::Integer(*value as isize)
    }
}
impl From<&usize> for Numeric {
    fn from(value: &usize) -> Self {
        Numeric::Integer(*value as isize)
    }
}
impl From<&i8> for Numeric {
    fn from(value: &i8) -> Self {
        Numeric::Integer(*value as isize)
    }
}
impl From<&i16> for Numeric {
    fn from(value: &i16) -> Self {
        Numeric::Integer(*value as isize)
    }
}
impl From<&i32> for Numeric {
    fn from(value: &i32) -> Self {
        Numeric::Integer(*value as isize)
    }
}
impl From<&i64> for Numeric {
    fn from(value: &i64) -> Self {
        Numeric::Integer(*value as isize)
    }
}
impl From<&i128> for Numeric {
    fn from(value: &i128) -> Self {
        Numeric::Integer(*value as isize)
    }
}
impl From<&isize> for Numeric {
    fn from(value: &isize) -> Self {
        Numeric::Integer(*value as isize)
    }
}
impl From<&f64> for Numeric {
    fn from(value: &f64) -> Self {
        Numeric::Float(*value as f64)
    }
}

impl From<Numeric> for u8 {
    fn from(value: Numeric) -> Self {
        match value {
            Numeric::Integer(i) => u8::try_from(i).unwrap_or_else(|_| {
                unreachable!("Conversion from Numeric to u8 resulted in overflow.")
            }),
            Numeric::Float(f) => {
                if (f >= u8::MIN as f64) && (f <= u8::MAX as f64) {
                    f as u8
                } else {
                    unreachable!("Conversion from Numeric to u8 resulted in overflow.")
                }
            }
        }
    }
}

impl From<Numeric> for u16 {
    fn from(value: Numeric) -> Self {
        match value {
            Numeric::Integer(i) => u16::try_from(i).unwrap_or_else(|_| {
                unreachable!("Conversion from Numeric to u16 resulted in overflow.")
            }),
            Numeric::Float(f) => {
                if (f >= u16::MIN as f64) && (f <= u16::MAX as f64) {
                    f as u16
                } else {
                    unreachable!("Conversion from Numeric to u16 resulted in overflow.")
                }
            }
        }
    }
}

impl From<Numeric> for u32 {
    fn from(value: Numeric) -> Self {
        match value {
            Numeric::Integer(i) => u32::try_from(i).unwrap_or_else(|_| {
                unreachable!("Conversion from Numeric to u32 resulted in overflow.")
            }),
            Numeric::Float(f) => {
                if (f >= u32::MIN as f64) && (f <= u32::MAX as f64) {
                    f as u32
                } else {
                    unreachable!("Conversion from Numeric to u32 resulted in overflow.")
                }
            }
        }
    }
}

impl From<Numeric> for u64 {
    fn from(value: Numeric) -> Self {
        match value {
            Numeric::Integer(i) => u64::try_from(i).unwrap_or_else(|_| {
                unreachable!("Conversion from Numeric to u64 resulted in overflow.")
            }),
            Numeric::Float(f) => {
                if (f >= u64::MIN as f64) && (f <= u64::MAX as f64) {
                    f as u64
                } else {
                    unreachable!("Conversion from Numeric to u64 resulted in overflow.")
                }
            }
        }
    }
}

impl From<Numeric> for u128 {
    fn from(value: Numeric) -> Self {
        match value {
            Numeric::Integer(i) => u128::try_from(i).unwrap_or_else(|_| {
                unreachable!("Conversion from Numeric to u128 resulted in overflow.")
            }),
            Numeric::Float(f) => {
                if (f >= u128::MIN as f64) && (f <= u128::MAX as f64) {
                    f as u128
                } else {
                    unreachable!("Conversion from Numeric to u128 resulted in overflow.")
                }
            }
        }
    }
}

impl From<Numeric> for usize {
    fn from(value: Numeric) -> Self {
        match value {
            Numeric::Integer(i) => usize::try_from(i).unwrap_or_else(|_| {
                unreachable!("Conversion from Numeric to usize resulted in overflow.")
            }),
            Numeric::Float(f) => {
                if (f >= usize::MIN as f64) && (f <= usize::MAX as f64) {
                    f as usize
                } else {
                    unreachable!("Conversion from Numeric to usize resulted in overflow.")
                }
            }
        }
    }
}

impl From<Numeric> for i8 {
    fn from(value: Numeric) -> Self {
        match value {
            Numeric::Integer(i) => i8::try_from(i).unwrap_or_else(|_| {
                unreachable!("Conversion from Numeric to i8 resulted in overflow.")
            }),
            Numeric::Float(f) => {
                if (f >= i8::MIN as f64) && (f <= i8::MAX as f64) {
                    f as i8
                } else {
                    unreachable!("Conversion from Numeric to i8 resulted in overflow.")
                }
            }
        }
    }
}

impl From<Numeric> for i16 {
    fn from(value: Numeric) -> Self {
        match value {
            Numeric::Integer(i) => i16::try_from(i).unwrap_or_else(|_| {
                unreachable!("Conversion from Numeric to i16 resulted in overflow.")
            }),
            Numeric::Float(f) => {
                if (f >= i16::MIN as f64) && (f <= i16::MAX as f64) {
                    f as i16
                } else {
                    unreachable!("Conversion from Numeric to i16 resulted in overflow.")
                }
            }
        }
    }
}

impl From<Numeric> for i32 {
    fn from(value: Numeric) -> Self {
        match value {
            Numeric::Integer(i) => i32::try_from(i).unwrap_or_else(|_| {
                unreachable!("Conversion from Numeric to i32 resulted in overflow.")
            }),
            Numeric::Float(f) => {
                if (f >= i32::MIN as f64) && (f <= i32::MAX as f64) {
                    f as i32
                } else {
                    unreachable!("Conversion from Numeric to i32 resulted in overflow.")
                }
            }
        }
    }
}

impl From<Numeric> for i64 {
    fn from(value: Numeric) -> Self {
        match value {
            Numeric::Integer(i) => i64::try_from(i).unwrap_or_else(|_| {
                unreachable!("Conversion from Numeric to i64 resulted in overflow.")
            }),
            Numeric::Float(f) => {
                if (f >= i64::MIN as f64) && (f <= i64::MAX as f64) {
                    f as i64
                } else {
                    unreachable!("Conversion from Numeric to i64 resulted in overflow.")
                }
            }
        }
    }
}

impl From<Numeric> for i128 {
    fn from(value: Numeric) -> Self {
        match value {
            Numeric::Integer(i) => i128::try_from(i).unwrap_or_else(|_| {
                unreachable!("Conversion from Numeric to i128 resulted in overflow.")
            }),
            Numeric::Float(f) => {
                if (f >= i128::MIN as f64) && (f <= i128::MAX as f64) {
                    f as i128
                } else {
                    unreachable!("Conversion from Numeric to i128 resulted in overflow.")
                }
            }
        }
    }
}

impl From<Numeric> for isize {
    fn from(value: Numeric) -> Self {
        match value {
            Numeric::Integer(i) => isize::try_from(i).unwrap_or_else(|_| {
                unreachable!("Conversion from Numeric to isize resulted in overflow.")
            }),
            Numeric::Float(f) => {
                if (f >= isize::MIN as f64) && (f <= isize::MAX as f64) {
                    f as isize
                } else {
                    unreachable!("Conversion from Numeric to isize resulted in overflow.")
                }
            }
        }
    }
}

impl From<Numeric> for f32 {
    fn from(value: Numeric) -> Self {
        let f = match value {
            Numeric::Integer(i) => i as f64,
            Numeric::Float(f) => f,
        };
        if (f >= f32::MIN as f64) && (f <= f32::MAX as f64) {
            f as f32
        } else {
            unreachable!("Conversion from Numeric to f32 resulted in overflow.")
        }
    }
}

impl From<Numeric> for f64 {
    fn from(value: Numeric) -> Self {
        match value {
            Numeric::Integer(i) => i as f64,
            Numeric::Float(f) => f,
        }
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

    pub fn to_float(self) -> f64 {
        match self {
            Numeric::Integer(value) => value as f64,
            Numeric::Float(value) => value,
        }
    }

    pub fn to_int(self) -> isize {
        match self {
            Numeric::Integer(value) => value,
            Numeric::Float(value) => value as isize,
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
        self * rhs.to_float()
    }
}

impl Mul<f64> for Numeric {
    type Output = f64;

    fn mul(self, rhs: f64) -> Self::Output {
        self * rhs
    }
}

impl Mul<&Numeric> for f64 {
    type Output = f64;

    fn mul(self, rhs: &Numeric) -> Self::Output {
        self * rhs.to_float()
    }
}

impl Mul<f64> for &Numeric {
    type Output = f64;

    fn mul(self, rhs: f64) -> Self::Output {
        self * rhs
    }
}

impl Div<Numeric> for f64 {
    type Output = f64;

    fn div(self, rhs: Numeric) -> Self::Output {
        self / rhs.to_float()
    }
}

impl Div<f64> for Numeric {
    type Output = f64;

    fn div(self, rhs: f64) -> Self::Output {
        self.to_float() / rhs
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

        assert_eq!(integer_a + integer_b, Numeric::Integer(3 as isize));
        assert_eq!(integer_a + float_a, Numeric::Float(4.0));
        assert_eq!(float_a + float_b, Numeric::Float(7.0));
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
    fn test_to_float() {
        let a = Numeric::from(3.0000000001);
        let b = 3.0000000001;

        assert_eq!(a.to_float(), b);
    }

    #[test]
    fn test_float_eq() {
        let a = 100.01;
        let b = 100.011;
        let epsilon_a = 0.1;
        let epsilon_b = 0.001;
        let epsilon_c = 0.0001;

        assert_eq!(Numeric::float_eq(a, b, epsilon_a), true);
        assert_eq!(Numeric::float_eq(a, b, epsilon_b), true);
        assert_eq!(Numeric::float_eq(a, b, epsilon_c), false);
    }
}
