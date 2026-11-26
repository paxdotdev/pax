use pax_kit::*;

pax_kit::runnable! { root: crate::ColorPickerExample }

#[pax]
#[file("color_picker.pax")]
pub struct ColorPickerExample {
    pub color: Property<Color>,
}
