use std::borrow::Borrow;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::fmt::{Formatter};
use std::ops::Mul;
use std::rc::Rc;
use uuid::Uuid;
use std::ffi::CString;

pub extern crate pax_macro;
pub use pax_macro::*;

#[macro_use]
extern crate lazy_static;
extern crate mut_static;

use mut_static::MutStatic;

pub struct TransitionQueueEntry<T> {
    pub global_frame_started: Option<usize>,
    pub duration_frames: u64,
    pub curve: EasingCurve,
    pub starting_value: T,
    pub ending_value: T,
}
/// An abstract Property that may be either: Literal,
/// a dynamic runtime Expression, or a Timeline-bound value
pub trait PropertyInstance<T: Default + Clone> {
    fn get(&self) -> &T;
    fn _get_vtable_id(&self) -> Option<u64>;

    fn set(&mut self, value: T);

    /// Used by engine to gain access to this property's transition queue
    fn _get_transition_manager(&mut self) -> Option<&mut TransitionManager<T>>;

    // fn get_eased_value(&self) -> Option<T>

    /// Immediately start transitioning from current value to the provided `new_value`,
    /// clearing the transition queue before doing so
    fn ease_to(&mut self, new_value: T, duration_frames: u64, curve: EasingCurve);

    /// Add a transition to the transition queue, which will execute
    /// after the current queue is complete.  The starting value for this new
    /// transition will be the final value upon completion of the current transition queue.
    fn ease_to_later(&mut self, new_value: T, duration_frames: u64, curve: EasingCurve);

}


// impl<T: Debug + Default + Clone + 'static> Debug for Box<dyn PropertyInstance<T>> {
//     fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
//         self.get().fmt(f)
//     }
// }

impl<T: Default + Clone + 'static> Default for Box<dyn PropertyInstance<T>> {
    fn default() -> Box<dyn PropertyInstance<T>> {
        Box::new(PropertyLiteral::new(Default::default()))
    }
}

impl<T: Default + Clone + 'static> Clone for Box<dyn PropertyInstance<T>> {
    fn clone(&self) -> Self {
        Box::clone(self)
    }
}

pub enum ArgsCoproduct {
    Render(ArgsRender),
    Click(ArgsClick),
}

pub type Property<T: Interpolatable> = Box<dyn PropertyInstance<T>>;



#[derive(Clone)]
pub struct ArgsRender {
    /// The current global engine tick count
    pub frames_elapsed: usize,
    /// The bounds of this element's container in px
    pub bounds: (f64, f64),
    // /// The number of adoptees passed to the current component (used by Stacker for auto cell-count calc; might be extended/adjusted for other use-cases)
    // pub adoptee_count: usize,
}


/// A Click occurs when the following sequence occurs:
///   0. mousedown
///   1. mouseup, must occur within the bounding box of the linked element
/// The contained `x` and `y` describe the coordinates relative to the linked element's bounding box
/// where the mousedown occurred.
#[derive(Clone)]
pub struct ArgsClick {
    pub x: f64,
    pub y: f64,
}

/// A Jab describes either a "click" (mousedown followed by mouseup), OR a
/// "tap" with one finger (singular fingerdown event).
/// Jabs are a useful alternative to most kinds of `Click` or `Tap` events,
/// when you want the same behavior for both to be contained in one place.
#[derive(Clone)]
pub struct ArgsJab {
    pub x: f64,
    pub y: f64,
}

/// A Size value that can be either a concrete pixel value
/// or a percent of parent bounds.
#[derive(Copy, Clone)]
pub enum Size {
    Pixels(f64),
    Percent(f64),
}

impl Size {
    fn get_property_manifest(field_name: &str, atomic_self_type: &str) -> PropertyManifest {
        let fully_qualified_path = module_path!().to_owned() + "::" + atomic_self_type;


        //TO implement (generate):  recursive invocations (alt: invoke flat list via tera-generated loop)
        //Represents a Property<> and its metadata:
        // -- fully qualified paths of all necessary imports
        // -- name of the property
        PropertyManifest {
            field_name: field_name.to_string(),
            fully_qualified_path,
        }
    }
}

#[derive(Copy, Clone)]
pub struct SizePixels(pub f64);
impl Default for SizePixels {
    fn default() -> Self {
        Self(150.0)
    }
}
impl Into<f64> for &SizePixels {
    fn into(self) -> f64 {
        self.0
    }
}
impl PartialEq<f64> for SizePixels {
    fn eq(&self, other: &f64) -> bool {
        self.0 == *other
    }
}
impl PartialEq<SizePixels> for f64 {
    fn eq(&self, other: &SizePixels) -> bool {
        other.0 == *self
    }
}

/// Coproduct for storing various kinds of function pointer,
/// needed to achieve compatibility with various native bridge mechanisms
pub enum PlatformSpecificLogger {
    Web(fn(&str)),
    MacOS(extern "C" fn(*const std::os::raw::c_char)),
}

pub struct Logger(PlatformSpecificLogger);

lazy_static! {
    static ref LOGGER: MutStatic<Logger> = MutStatic::new();
}

pub fn register_logger(logger: PlatformSpecificLogger) {
    LOGGER.borrow().set(Logger(logger)).unwrap();
}

/// Log to the appropriate native logging mechanism
/// Most often called as `pax::log("some message")`
pub fn log(msg: &str) {
    let logging_variant = &(LOGGER.borrow().read().expect("Logger isn't registered").0);
    match logging_variant {
        PlatformSpecificLogger::Web(closure) => {
            closure(msg)
        },
        PlatformSpecificLogger::MacOS(closure) => {
            let msg = CString::new(msg).unwrap();
            unsafe {(closure)(msg.as_ptr())};
        }
    }
}

impl Mul for Size {
    type Output = Size;

    fn mul(self, rhs: Self) -> Self::Output {
        match self {
            Size::Pixels(px0) => {
                match rhs {
                    //multiplying two pixel values adds them,
                    //in the sense of multiplying two affine translations.
                    //this might be wildly unexpected in some cases, so keep an eye on this and
                    //revisit whether to support Percent values in anchor calcs (could rescind)
                    Size::Pixels(px1) => {
                        Size::Pixels(px0 + px1)
                    }
                    Size::Percent(pc1) => {
                        Size::Pixels(px0 * pc1)
                    }
                }
            }
            Size::Percent(pc0) => {
                match rhs {
                    Size::Pixels(px1) => {
                        Size::Pixels(pc0 * px1)
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
    rotate: Option<Box<dyn PropertyInstance<f64>>>
}

// More than just a tuble of (Size, Size),
// Size2D wraps up Properties as well to make it easy
// to declare expressable Size properties
pub type Size2D = Rc<RefCell<[Box<dyn PropertyInstance<Size>>; 2]>>;




/// A sugary representation of an Affine transform+, including
/// `anchor` and `align` as layout-computed properties.
///
/// `translate` represents an (x,y) affine translation
/// `scale`     represents an (x,y) non-uniform affine scale
/// `rotate`    represents a (z) affine rotation (intuitive 2D rotation)
/// `anchor`    represents the "(0,0)" point of the render node as it relates to its own bounding box.
///             By default that's the top-left of the element, but `anchor` allows that
///             to be offset either by a pixel or percentage-of-element-size
///             for each of (x,y)
/// `align`     the offset of this element's `anchor` as it relates to the element's parent.
///             By default this is the top-left corner of the parent container,
///             but can be set to be any value [0,1] for each of (x,y), representing
///             the percentage (between 0.0 and 1.0) multiplied by the parent container size.
///             For example, an align of (0.5, 0.5) will center an element's `anchor` point both vertically
///             and horizontally within the parent container.  Combined with an anchor of (Size::Percent(50.0), Size::Percent(50.0)),
///             an element will appear fully centered within its parent.
#[derive(Default, Clone)]
pub struct Transform2D { //Literal
    pub previous: Option<Box<Transform2D>>,
    pub rotate: Option<f64>, ///over z axis
    pub translate: Option<[f64; 2]>,
    pub anchor: Option<[Size; 2]>,
    pub align: Option<[Size; 2]>,
    pub scale: Option<[f64; 2]>,
}

impl Mul for Transform2D {
    type Output = Transform2D;

    fn mul(self, rhs: Self) -> Self::Output {
        let mut ret = rhs.clone();
        ret.previous = Some(Box::new(self));
        ret
    }
}

impl Transform2D {
    ///Scale coefficients (1.0 == 100%) over x-y plane
    pub fn scale(x: f64, y: f64) -> Self {
        let mut ret  = Transform2D::default();
        ret.scale = Some([x, y]);
        ret
    }
    ///Rotation over z axis
    pub fn rotate(z: f64) -> Self {
        let mut ret  = Transform2D::default();
        ret.rotate = Some(z);
        ret
    }
    ///Translation across x-y plane, pixels
    pub fn translate(x: f64, y: f64) -> Self {
        let mut ret  = Transform2D::default();
        ret.translate = Some([x, y]);
        ret
    }
    ///Describe alignment within parent bounding box, as a starting point before
    /// affine transformations are applied
    pub fn align(x: Size, y: Size) -> Self {
        let mut ret  = Transform2D::default();
        ret.align = Some([x, y]);
        ret
    }
    ///Describe alignment of the (0,0) position of this element as it relates to its own bounding box
    pub fn anchor(x: Size, y: Size) -> Self {
        let mut ret  = Transform2D::default();
        ret.anchor = Some([x, y]);
        ret
    }

    pub fn default_wrapped() -> Rc<RefCell<dyn PropertyInstance<Self>>> {
        Rc::new(RefCell::new(PropertyLiteral::new(Transform2D::default())))
    }
}



pub struct TransitionManager<T> {
    pub queue: VecDeque<TransitionQueueEntry<T>>,
    pub value: Option<T>,
}

impl<T> TransitionManager<T> {
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
            value: None,
        }
    }
}

/// The Literal form of a Property: a bare literal value with support for easing/interpolation
pub struct PropertyLiteral<T> {
    value: T,
    transition_manager: TransitionManager<T>,
}


impl<T> Into<Box<dyn PropertyInstance<T>>> for PropertyLiteral<T>
where T: Default + Clone + 'static {
    fn into(self) -> Box<dyn PropertyInstance<T>> {
        Box::new(self)
    }
}


impl<T: Clone> PropertyLiteral<T> {
    pub fn new(value: T) -> Self {
        PropertyLiteral {
            value,
            transition_manager: TransitionManager::new(),
        }
    }


}
impl<T: Default + Clone> PropertyInstance<T> for PropertyLiteral<T> {
    fn get(&self) -> &T {
        &self.value
    }

    fn _get_vtable_id(&self) -> Option<u64> {
        None
    }

    fn set(&mut self, value: T) {
        self.value = value;
    }

    //TODO: when trait fields land in Rust, DRY this implementation vs. other <T: PropertyInstance> implementations
    fn ease_to(&mut self, new_value: T, duration_frames: u64, curve: EasingCurve) {
        self.transition_manager.value = Some(self.get().clone());
        &self.transition_manager.queue.clear();
        &self.transition_manager.queue.push_back(TransitionQueueEntry {
            global_frame_started: None,
            duration_frames,
            curve,
            starting_value: self.value.clone(),
            ending_value: new_value
        });
    }

    fn ease_to_later(&mut self, new_value: T, duration_frames: u64, curve: EasingCurve) {
        if let None = self.transition_manager.value {
            //handle case where transition queue is empty -- a None value gets skipped, so populate it with Some
            self.transition_manager.value = Some(self.get().clone());
        }
        &self.transition_manager.queue.push_back(TransitionQueueEntry {
            global_frame_started: None,
            duration_frames,
            curve,
            starting_value: self.value.clone(),
            ending_value: new_value
        });
    }

    fn _get_transition_manager(&mut self) -> Option<&mut TransitionManager<T>> {
        if let None = self.transition_manager.value {
            None
        }else {
            Some(&mut self.transition_manager)
        }
    }
}

pub enum EasingCurve {
    Linear,
    InQuad,
    OutQuad,
    InBack,
    OutBack,
    InOutBack,
    Custom(Box<dyn Fn(f64) -> f64>),
}

struct EasingEvaluators {}
impl EasingEvaluators {
    fn linear(t: f64) -> f64 {
        t
    }
    fn none(t: f64) -> f64 {
        if t == 1.0 { 1.0 } else { 0.0 }
    }
    fn in_quad(t: f64) -> f64 {
        t * t
    }
    fn out_quad(t: f64) -> f64 {
        1.0 - (1.0 - t) * (1.0 - t)
    }
    fn in_back(t: f64) -> f64 {
        const C1: f64 = 1.70158;
        const C3: f64 = C1 + 1.00;
        C3 * t * t * t - C1 * t * t
    }
    fn out_back(t: f64) -> f64 {
        const C1: f64 = 1.70158;
        const C3: f64 = C1 + 1.00;
        1.0 + C3 * (t - 1.0).powi(3) + C1 * (t - 1.0).powi(2)
    }

    fn in_out_back(t: f64) -> f64 {
        const C1: f64 = 1.70158;
        const C2 : f64 = C1 * 1.525;
        if t < 0.5 {
            ((2.0 * t).powi(2) * ((C2 + 1.0) * 2.0 * t - C2)) / 2.0
        } else {
            ((2.0 * t - 2.0).powi(2) * ((C2 + 1.0) * (t * 2.0 - 2.0) + C2) + 2.0) / 2.0
        }
    }
}

impl EasingCurve {
    //for a time on the unit interval `t ∈ [0,1]`, given a value `t`,
    // find the interpolated value `vt` between `v0` and `v1` given the self-contained easing curve
    pub fn interpolate<T: Interpolatable>(&self, v0: &T, v1: &T, t: f64) -> T /*vt*/ {
        let multiplier = match self {
            EasingCurve::Linear => {
                EasingEvaluators::linear(t)
            }
            EasingCurve::InQuad => {
                EasingEvaluators::in_quad(t)
            }
            EasingCurve::OutQuad => {
                EasingEvaluators::out_quad(t)
            }
            EasingCurve::InBack => {
                EasingEvaluators::in_back(t)
            }
            EasingCurve::OutBack => {
                EasingEvaluators::out_back(t)
            }
            EasingCurve::InOutBack => {
                EasingEvaluators::in_out_back(t)
            }
            EasingCurve::Custom(evaluator) => {
                (*evaluator)(t)
            }
        };

        v0.interpolate( v1, multiplier)
    }
}

pub trait Interpolatable
where Self : Sized + Clone //Clone used for default implementation of `interpolate`
{
    //default implementation acts like a `None` ease — that is,
    //the first value is simply returned.
    fn interpolate(&self, other: &Self, t: f64) -> Self {
        self.clone()
    }
}

impl Interpolatable for f64 {
    fn interpolate(&self, other: &f64, t: f64) -> f64 {
        self + (*other - self) * t
    }
}

impl Interpolatable for usize {
    fn interpolate(&self, other: &usize, t: f64) -> usize {
        (*self as f64 + (*other - self) as f64 * t) as usize
    }
}

impl Interpolatable for isize {
    fn interpolate(&self, other: &isize, t: f64) -> isize {
        (*self as f64 + (*other - self) as f64 * t) as isize
    }
}

impl Interpolatable for String {}


pub struct Timeline {
    pub playhead_position: usize,
    pub frame_count: usize,
    pub is_playing: bool,
}
