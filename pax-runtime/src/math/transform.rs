use std::{marker::PhantomData, ops::Mul};

//-----------------------------------------------------------
// Pax matrix/transform class heavily borrows from kurbos
// transform impl (copy/pasted initially with some modifications)
// curbo crate: https://www.michaelfbryan.com/arcs/kurbo/index.html
// original source code: https://www.michaelfbryan.com/arcs/src/kurbo/affine.rs.html#10
// kurbo is distributed under the following (MIT) license:
// "Copyright (c) 2018 Raph Levien

// Permission is hereby granted, free of charge, to any
// person obtaining a copy of this software and associated
// documentation files (the "Software"), to deal in the
// Software without restriction, including without
// limitation the rights to use, copy, modify, merge,
// publish, distribute, sublicense, and/or sell copies of
// the Software, and to permit persons to whom the Software
// is furnished to do so, subject to the following
// conditions:
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions
// of the Software.

// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
// ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
// TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
// PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
// SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
// CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
// OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
// IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
// DEALINGS IN THE SOFTWARE."
//-----------------------------------------------------------

use super::{Space, Vector2};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transform2<WFrom, WTo = WFrom> {
    m: [f64; 6],
    _panthom_from: PhantomData<WFrom>,
    _panthom_to: PhantomData<WTo>,
}

impl<WFrom: Space, WTo: Space> Transform2<WFrom, WTo> {
    pub fn new(m: [f64; 6]) -> Self {
        Self {
            m,
            _panthom_from: PhantomData,
            _panthom_to: PhantomData,
        }
    }

    pub fn unit() -> Self {
        Self::new([1.0, 0.0, 0.0, 1.0, 0.0, 0.0])
    }

    pub fn scale(s: f64) -> Self {
        Self::new([s, 0.0, 0.0, s, 0.0, 0.0])
    }

    pub fn rotate(th: f64) -> Self {
        let (s, c) = th.sin_cos();
        Self::new([c, s, -s, c, 0.0, 0.0])
    }

    pub fn translate(p: Vector2<WFrom>) -> Self {
        Self::new([1.0, 0.0, 0.0, 1.0, p.x, p.y])
    }

    pub fn determinant(self) -> f64 {
        self.m[0] * self.m[3] - self.m[1] * self.m[2]
    }

    /// Produces NaN values when the determinant is zero.
    pub fn inverse(self) -> Transform2<WTo, WFrom> {
        let inv_det = self.determinant().recip();
        Transform2::<WTo, WFrom>::new([
            inv_det * self.m[3],
            -inv_det * self.m[1],
            -inv_det * self.m[2],
            inv_det * self.m[0],
            inv_det * (self.m[2] * self.m[5] - self.m[3] * self.m[4]),
            inv_det * (self.m[1] * self.m[4] - self.m[0] * self.m[5]),
        ])
    }
}

impl<W1: Space, W2: Space, W3: Space> Mul<Transform2<W1, W2>> for Transform2<W2, W3> {
    type Output = Transform2<W1, W3>;

    fn mul(self, rhs: Transform2<W1, W2>) -> Self::Output {
        Self::Output::new([
            self.m[0] * rhs.m[0] + self.m[2] * rhs.m[1],
            self.m[1] * rhs.m[0] + self.m[3] * rhs.m[1],
            self.m[0] * rhs.m[2] + self.m[2] * rhs.m[3],
            self.m[1] * rhs.m[2] + self.m[3] * rhs.m[3],
            self.m[0] * rhs.m[4] + self.m[2] * rhs.m[5] + self.m[4],
            self.m[1] * rhs.m[4] + self.m[3] * rhs.m[5] + self.m[5],
        ])
    }
}
