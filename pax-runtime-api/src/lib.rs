use std::borrow::{Borrow, BorrowMut};
use std::cell::RefCell;
use std::ops::{Deref, Mul};
use std::rc::Rc;
use kurbo::Affine;
use uuid::Uuid;

#[macro_use]
extern crate lazy_static;
extern crate mut_static;

use mut_static::MutStatic;

// /// An abstract Property that may be either: Literal,
// /// a dynamic runtime Expression, or a Timeline-bound value
pub trait Property<T> {
    fn get(&self) -> &T;
    fn get_id(&self) -> Option<&str>;
    fn set(&mut self, value: T);
}

//keep an eye on perf. here — might be more sensible to use something like
//a monotonically increasing counter of i32 instead of String UUIDs.  Might require coordinating between
//code-genned IDs in code-gen and dynamically generated IDs here to avoid dupes.
pub fn generate_unique_id() -> String {
    Uuid::new_v4().to_string()
}

pub enum ArgsCoproduct {
    Tick(ArgsTick),
    Click(ArgsClick),
}

#[derive(Clone)]
pub struct ArgsTick {
    pub frame: i64,
}

#[derive(Clone)]
pub struct ArgsClick {
    x: f64,
    y: f64,
}

/// A size value that can be either a concrete pixel value
/// or a percent of parent bounds.
#[derive(Copy, Clone)]
pub enum Size {
    Pixel(f64),
    Percent(f64),
}

pub struct Logger(fn(&str));

lazy_static! {
    static ref LOGGER: MutStatic<Logger> = MutStatic::new();
}

pub fn register_logger(logger: fn(&str)) {
    LOGGER.borrow().set(Logger(logger));
}

pub fn log(msg: &str) {
    //TODO: instead of unwrap, handle case where logger isn't registered
    unsafe {
        (LOGGER.borrow().read().unwrap().0)(msg);
    }
}

impl Mul for Size {
    type Output = Size;

    fn mul(self, rhs: Self) -> Self::Output {
        match self {
            Size::Pixel(px0) => {
                match rhs {
                    //multiplying two pixel values adds them,
                    //in the sense of multiplying two affine translations.
                    //this might be wildly unexpected in some cases, so keep an eye on this and
                    //revisit whether to support Percent values in origin calcs (could rescind)
                    Size::Pixel(px1) => {
                        Size::Pixel(px0 + px1)
                    }
                    Size::Percent(pc1) => {
                        Size::Pixel(px0 * pc1)
                    }
                }
            }
            Size::Percent(pc0) => {
                match rhs {
                    Size::Pixel(px1) => {
                        Size::Pixel(pc0 * px1)
                    }
                    Size::Percent(pc1) => {
                        Size::Percent(pc0 * pc1)
                    }
                }
            }
        }
    }
}


/// TODO: revisit if 100% is the most ergonomic default size (remember Dreamweaver)
impl Default for Size {
    fn default() -> Self {
        Self::Percent(100.0)
    }
}

pub struct TransformInstance {
    rotate: Option<Box<dyn Property<f64>>>
}

// More than just a tuble of (Size, Size),
// Size2D wraps up Properties as well to make it easy
// to declare expressable Size properties
pub type Size2D = Rc<RefCell<[Box<dyn Property<Size>>; 2]>>;


#[derive(Default, Clone)]
pub struct Transform { //Literal
    pub previous: Option<Box<Transform>>,
    pub rotate: Option<f64>, ///over z axis
    pub translate: Option<[f64; 2]>,
    pub origin: Option<[Size; 2]>,
    pub align: Option<[f64; 2]>,
    pub scale: Option<[f64; 2]>,
}

impl Mul for Transform {
    type Output = Transform;

    fn mul(self, rhs: Self) -> Self::Output {
        let mut ret = rhs.clone();
        ret.previous = Some(Box::new(self));
        ret
    }
}

impl Transform {
    ///Scale coefficients (1.0 == 100%) over x-y plane
    pub fn scale(x: f64, y: f64) -> Self {
        let mut ret  = Transform::default();
        ret.scale = Some([x, y]);
        ret
    }
    ///Rotation over z axis
    pub fn rotate(z: f64) -> Self {
        let mut ret  = Transform::default();
        ret.rotate = Some(z);
        ret
    }
    ///Translation across x-y plane, pixels
    pub fn translate(x: f64, y: f64) -> Self {
        let mut ret  = Transform::default();
        ret.translate = Some([x, y]);
        ret
    }
    ///Describe alignment within parent bounding box, as a starting point before
    /// affine transformations are applied
    pub fn align(x: f64, y: f64) -> Self {
        let mut ret  = Transform::default();
        ret.align = Some([x, y]);
        ret
    }
    ///Describe alignment of the (0,0) position of this element as it relates to its own bounding box
    pub fn origin(x: Size, y: Size) -> Self {
        let mut ret  = Transform::default();
        ret.origin = Some([x, y]);
        ret
    }
}

/// The Literal form of a Property: a bare literal value
pub struct PropertyLiteral<T> {
    pub value: T,
}

impl<T> Property<T> for PropertyLiteral<T> {
    fn get(&self) -> &T {
        &self.value
    }

    fn get_id(&self) -> Option<&str> {
        None
    }

    fn set(&mut self, value: T) {
        self.value = value;
    }
}

impl<T> PropertyLiteral<T> {
    pub fn new(value: T) -> Box<Self> {
        Box::new(Self {
            value,
        })
    }

}

pub struct PropertyTimeline {
    pub id: String,
    pub starting_value: Box<dyn Property<f64>>,
    pub timeline_segments: Vec<TimelineSegment>,
    pub cached_evaluated_value: f64,
}

impl Property<f64> for PropertyTimeline {
    fn get(&self) -> &f64 {
        &self.cached_evaluated_value
    }

    fn get_id(&self) -> Option<&str> {
        Some(self.id.as_str())
    }

    fn set(&mut self, value: f64) {
        self.cached_evaluated_value = value;
    }
}


pub struct TimelineSegment {
    pub curve_in: Box<dyn Easible>,
    pub ending_value: Box<dyn Property<f64>>,
    pub ending_frame_inclusive: usize,
}

// Author's note:  zb deliberated between "Easable," "Easeable", and "Easible".
// Given that none of the above could be found in written reference, zb decided
// on Easible for 'able to be eased' c.f. 'audible', 'able to be heard'
pub trait Easible {
    /// Map the domain x [0,1] to the range y [all f64]
    fn map(&self, x: f64) -> f64;
}

pub struct NoneEasingCurve {}

impl Easible for NoneEasingCurve {
    fn map(&self, _x: f64) -> f64 {
        0.0
    }
}


pub struct LinearEasingCurve {}

impl Easible for LinearEasingCurve {
    fn map(&self, x: f64) -> f64 {
        x
    }
}

pub struct InQuadEasingCurve {}

impl Easible for InQuadEasingCurve{
    fn map(&self, x: f64) -> f64 {
        x * x
    }
}

pub struct OutQuadEasingCurve {}

impl Easible for OutQuadEasingCurve{
    fn map(&self, x: f64) -> f64 {
        1.0 - (1.0 - x) * (1.0 - x)
    }
}

pub struct InBackEasingCurve {}

impl Easible for InBackEasingCurve{
    fn map(&self, x: f64) -> f64 {
        const C1: f64 = 1.70158;
        const C3: f64 = C1 + 1.00;
        C3 * x * x * x - C1 * x * x
    }
}

pub struct OutBackEasingCurve {}

impl Easible for OutBackEasingCurve{
    fn map(&self, x: f64) -> f64 {
        const C1: f64 = 1.70158;
        const C3: f64 = C1 + 1.00;
        1.0 + C3 * (x - 1.0).powi(3) + C1 * (x - 1.0).powi(2)
    }
}

pub struct InOutBackEasingCurve {}

impl Easible for InOutBackEasingCurve{
    fn map(&self, x: f64) -> f64 {
        const C1: f64 = 1.70158;
        const C2 : f64 = C1 * 1.525;
        if x < 0.5 {
            ((2.0 * x).powi(2) * ((C2 + 1.0) * 2.0 * x - C2)) / 2.0
        } else {
            ((2.0 * x - 2.0).powi(2) * ((C2 + 1.0) * (x * 2.0 - 2.0) + C2) + 2.0) / 2.0
        }
    }
}

pub struct EasingCurve {}

impl EasingCurve {
    pub fn none() -> Box<dyn Easible> {
        Box::new(NoneEasingCurve {})
    }
    pub fn linear() -> Box<dyn Easible> {
        Box::new(LinearEasingCurve {})
    }
    pub fn in_quad() -> Box<dyn Easible> {
        Box::new(InQuadEasingCurve {})
    }
    pub fn out_quad() -> Box<dyn Easible> {
        Box::new(OutQuadEasingCurve {})
    }
    pub fn in_back() -> Box<dyn Easible> {
        Box::new(InBackEasingCurve {})
    }
    pub fn out_back() -> Box<dyn Easible> {
        Box::new(OutBackEasingCurve {})
    }
    pub fn in_out_back() -> Box<dyn Easible> {
        Box::new(InOutBackEasingCurve {})
    }
}


pub struct Timeline {
    pub playhead_position: usize,
    pub frame_count: usize,
    pub is_playing: bool,
}
