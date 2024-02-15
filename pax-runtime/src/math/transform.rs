use std::marker::PhantomData;

use super::{vector::Vector2, Space};

pub struct Transform2<WFrom, WTo> {
    m: [f64; 6],
    _panthom_from: PhantomData<WFrom>,
    _panthom_to: PhantomData<WTo>,
}

impl<WFrom: Space, WTo: Space> Transform2<WFrom, WTo> {
    pub fn from_coefs(m: [f64; 6]) -> Self {
        Self {
            m,
            _panthom_from: PhantomData,
            _panthom_to: PhantomData,
        }
    }
}
