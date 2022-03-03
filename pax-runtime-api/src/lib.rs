use std::borrow::Borrow;
use std::cell::RefCell;
use std::ops::Mul;
use std::rc::Rc;
use uuid::Uuid;

#[macro_use]
extern crate lazy_static;
extern crate mut_static;

use mut_static::MutStatic;

// /// An abstract Property that may be either: Literal,
// /// a dynamic runtime Expression, or a Timeline-bound value
pub trait Property<T: Default> {
    fn get(&self) -> &T;
    fn _get_vtable_id(&self) -> Option<&str>;
    fn set(&mut self, value: T);
}

impl<T: Default + 'static> Default for Box<dyn Property<T>> {
    fn default() -> Box<dyn Property<T>> {
        Box::new(PropertyLiteral(Default::default()))
    }
}

//keep an eye on perf. here — might be more sensible to use something like
//a monotonically increasing counter of i32 instead of String UUIDs.  Might require coordinating between
//code-genned IDs in code-gen and dynamically generated IDs here to avoid dupes.
pub fn mint_unique_id() -> String {
    Uuid::new_v4().to_string()
}

pub enum ArgsCoproduct {
    Tick(ArgsTick),
    Click(ArgsClick),
}

#[derive(Clone)]
pub struct ArgsTick {
    pub frame: usize,
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
    LOGGER.borrow().set(Logger(logger)).unwrap();
}

pub fn log(msg: &str) {
    (LOGGER.borrow().read().unwrap().0)(msg);
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

    pub fn default_wrapped() -> Rc<RefCell<dyn Property<Self>>> {
        Rc::new(RefCell::new(PropertyLiteral(Transform::default())))
    }
}

/// The Literal form of a Property: a bare literal value
pub struct PropertyLiteral<T>(pub T);


impl<T> Into<Box<dyn Property<T>>> for PropertyLiteral<T>
where T: Default + 'static {
    fn into(self) -> Box<dyn Property<T>> {
        Box::new(self)
    }
}

impl<T: Default> Property<T> for PropertyLiteral<T> {
    fn get(&self) -> &T {
        &self.0
    }

    fn _get_vtable_id(&self) -> Option<&str> {
        None
    }

    fn set(&mut self, value: T) {
        self.0 = value;
    }
}

impl<T> PropertyLiteral<T> {
    pub fn new(value: T) -> Box<Self> {
        Box::new(Self(value))
    }

}

pub trait Tweenable {
    /// Map the domain x [0,1] to the range y [all f64]
    fn map(&self, x: f64) -> f64;
}

pub struct NoneEasingCurve {}

impl Tweenable for NoneEasingCurve {
    fn map(&self, _x: f64) -> f64 {
        0.0
    }
}


pub struct LinearEasingCurve {}

impl Tweenable for LinearEasingCurve {
    fn map(&self, x: f64) -> f64 {
        x
    }
}

pub struct InQuadEasingCurve {}

impl Tweenable for InQuadEasingCurve{
    fn map(&self, x: f64) -> f64 {
        x * x
    }
}

pub struct OutQuadEasingCurve {}

impl Tweenable for OutQuadEasingCurve{
    fn map(&self, x: f64) -> f64 {
        1.0 - (1.0 - x) * (1.0 - x)
    }
}

pub struct InBackEasingCurve {}

impl Tweenable for InBackEasingCurve{
    fn map(&self, x: f64) -> f64 {
        const C1: f64 = 1.70158;
        const C3: f64 = C1 + 1.00;
        C3 * x * x * x - C1 * x * x
    }
}

pub struct OutBackEasingCurve {}

impl Tweenable for OutBackEasingCurve{
    fn map(&self, x: f64) -> f64 {
        const C1: f64 = 1.70158;
        const C3: f64 = C1 + 1.00;
        1.0 + C3 * (x - 1.0).powi(3) + C1 * (x - 1.0).powi(2)
    }
}

pub struct InOutBackEasingCurve {}

impl Tweenable for InOutBackEasingCurve{
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
    pub fn none() -> Box<dyn Tweenable> {
        Box::new(NoneEasingCurve {})
    }
    pub fn linear() -> Box<dyn Tweenable> {
        Box::new(LinearEasingCurve {})
    }
    pub fn in_quad() -> Box<dyn Tweenable> {
        Box::new(InQuadEasingCurve {})
    }
    pub fn out_quad() -> Box<dyn Tweenable> {
        Box::new(OutQuadEasingCurve {})
    }
    pub fn in_back() -> Box<dyn Tweenable> {
        Box::new(InBackEasingCurve {})
    }
    pub fn out_back() -> Box<dyn Tweenable> {
        Box::new(OutBackEasingCurve {})
    }
    pub fn in_out_back() -> Box<dyn Tweenable> {
        Box::new(InOutBackEasingCurve {})
    }
}


pub struct Timeline {
    pub playhead_position: usize,
    pub frame_count: usize,
    pub is_playing: bool,
}
