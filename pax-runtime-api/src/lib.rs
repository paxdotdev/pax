
pub struct Property<T: Default> {
    value: T,
}

impl<T: Default> Property<T> {
    pub fn new() -> Self {
        Self {value: T::default()}
    }
    pub fn from(value: T) -> Self {
        Self {value}
    }
    pub fn get(&self) -> &T {
        &self.value
    }
    pub fn set(&mut self, new: T) {
        self.value = new;
    }
}


/// A size value that can be either a concrete pixel value
/// or a percent of parent bounds.
#[derive(Copy, Clone)]
pub enum Size {
    Pixel(f64),
    Percent(f64),
}

/// TODO: revisit if 100% is the most ergonomic default size (remember Dreamweaver)
impl Default for Size {
    fn default() -> Self {
        Self::Percent(100.0)
    }
}