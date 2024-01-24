pub mod numeric;

use std::borrow::Borrow;
use std::collections::VecDeque;
use std::ffi::CString;

use std::ops::{Add, Deref, Mul, Neg};

#[macro_use]
extern crate lazy_static;
extern crate mut_static;

pub use crate::numeric::Numeric;
use kurbo::{BezPath, Rect};
use mut_static::MutStatic;
use pax_message::{ModifierKeyMessage, MouseButtonMessage, TouchMessage};
use piet::{ImageBuf, PaintBrush};

#[cfg(feature = "designtime")]
use pax_designtime::DesigntimeManager;

#[cfg(feature = "designtime")]
use std::rc::Rc;

#[cfg(feature = "designtime")]
use std::cell::RefCell;

pub struct TransitionQueueEntry<T> {
    pub global_frame_started: Option<usize>,
    pub duration_frames: u64,
    pub curve: EasingCurve,
    pub starting_value: T,
    pub ending_value: T,
}

#[cfg(debug_assertions)]
impl<T: std::fmt::Debug> std::fmt::Debug for TransitionQueueEntry<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TransitionQueueEntry")
            .field("global_frame_started", &self.global_frame_started)
            .field("duration_frames", &self.duration_frames)
            .field("starting_value", &self.starting_value)
            .field("ending_value", &self.ending_value)
            .finish()
    }
}

/// An abstract Property that may be either: Literal,
/// a dynamic runtime Expression, or a Timeline-bound value
pub trait PropertyInstance<T: Default + Clone> {
    fn get(&self) -> &T;
    fn get_mut(&mut self) -> &mut T;
    fn _get_vtable_id(&self) -> Option<usize>;

    fn set(&mut self, value: T);

    /// Used by engine to gain access to this property's transition queue
    fn _get_transition_manager(&mut self) -> Option<&mut TransitionManager<T>>;

    /// Immediately start transitioning from current value to the provided `new_value`,
    /// clearing the transition queue before doing so
    fn ease_to(&mut self, new_value: T, duration_frames: u64, curve: EasingCurve);

    /// Add a transition to the transition queue, which will execute
    /// after the current queue is complete.  The starting value for this new
    /// transition will be the final value upon completion of the current transition queue.
    fn ease_to_later(&mut self, new_value: T, duration_frames: u64, curve: EasingCurve);

    //Wishlist:
    // to_default: set back to default value
    // ease_to_default: set back to default value via interpolation
    // ^ for the above, consider the transient changes to dirty-DAG when we switch between a Literal and Expression.
}

#[cfg(debug_assertions)]
impl<T: Default + std::fmt::Debug + Clone + 'static> std::fmt::Debug
    for Box<dyn PropertyInstance<T>>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.get().fmt(f)
    }
}

impl<T: Default + Clone + 'static> Default for Box<dyn PropertyInstance<T>> {
    fn default() -> Box<dyn PropertyInstance<T>> {
        Box::new(PropertyLiteral::new(Default::default()))
    }
}

impl<T: Default + Clone + 'static> Clone for Box<dyn PropertyInstance<T>> {
    fn clone(&self) -> Self {
        Box::new(PropertyLiteral::new(self.deref().get().clone()))
    }
}

pub type Property<T> = Box<dyn PropertyInstance<T>>;

#[derive(Clone)]
#[cfg_attr(debug_assertions, derive(Debug))]
pub struct NodeContext {
    /// The current global engine tick count
    pub frames_elapsed: usize,
    /// The bounds of this element's immediate container (parent) in px
    pub bounds_parent: (f64, f64),
    /// The bounds of this element in px
    pub bounds_self: (f64, f64),

    #[cfg(feature = "designtime")]
    pub designtime: Rc<RefCell<DesigntimeManager>>,
}

// Unified events

/// A Clap describes either a "click" (mousedown followed by mouseup), OR a
/// "tap" with one finger (singular fingerdown event).
/// Claps are a useful alternative to most kinds of `Click` or `Tap` events,
/// when you want the same behavior for both to be contained in one place.
#[derive(Clone)]
pub struct ArgsClap {
    pub x: f64,
    pub y: f64,
}

/// Scroll occurs when a frame is translated vertically or horizontally
/// Can be both by touch, mouse or keyboard
/// The contained `delta_x` and `delta_y` describe the horizontal and vertical translation of
/// the frame
#[derive(Clone)]
pub struct ArgsScroll {
    pub delta_x: f64,
    pub delta_y: f64,
}

// Touch Events

/// Represents a single touch point.
#[derive(Clone)]
pub struct Touch {
    pub x: f64,
    pub y: f64,
    pub identifier: i64,
    pub delta_x: f64,
    pub delta_y: f64,
}

impl From<&TouchMessage> for Touch {
    fn from(value: &TouchMessage) -> Self {
        Touch {
            x: value.x,
            y: value.y,
            identifier: value.identifier,
            delta_x: value.delta_x,
            delta_y: value.delta_x,
        }
    }
}

/// A TouchStart occurs when the user touches an element.
/// The contained `touches` represent a list of touch points.
#[derive(Clone)]
pub struct ArgsTouchStart {
    pub touches: Vec<Touch>,
}

/// A TouchMove occurs when the user moves while touching an element.
/// The contained `touches` represent a list of touch points.
#[derive(Clone)]
pub struct ArgsTouchMove {
    pub touches: Vec<Touch>,
}

/// A TouchEnd occurs when the user stops touching an element.
/// The contained `touches` represent a list of touch points.
#[derive(Clone)]
pub struct ArgsTouchEnd {
    pub touches: Vec<Touch>,
}

// Keyboard Events

/// Common properties in keyboard events.
#[derive(Clone)]
pub struct KeyboardEventArgs {
    pub key: String,
    pub modifiers: Vec<ModifierKey>,
    pub is_repeat: bool,
}

/// User is pressing a key.
#[derive(Clone)]
pub struct ArgsKeyDown {
    pub keyboard: KeyboardEventArgs,
}

/// User has released a key.
#[derive(Clone)]
pub struct ArgsKeyUp {
    pub keyboard: KeyboardEventArgs,
}

/// User presses a key that displays a character (alphanumeric or symbol).
#[derive(Clone)]
pub struct ArgsKeyPress {
    pub keyboard: KeyboardEventArgs,
}

// Mouse Events

/// Common properties in mouse events.
#[derive(Clone)]
pub struct MouseEventArgs {
    pub x: f64,
    pub y: f64,
    pub button: MouseButton,
    pub modifiers: Vec<ModifierKey>,
}

#[derive(Clone)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Unknown,
}

impl From<MouseButtonMessage> for MouseButton {
    fn from(value: MouseButtonMessage) -> Self {
        match value {
            MouseButtonMessage::Left => MouseButton::Left,
            MouseButtonMessage::Right => MouseButton::Right,
            MouseButtonMessage::Middle => MouseButton::Middle,
            MouseButtonMessage::Unknown => MouseButton::Unknown,
        }
    }
}

#[derive(Clone)]
pub enum ModifierKey {
    Shift,
    Control,
    Alt,
    Command,
}

impl From<&ModifierKeyMessage> for ModifierKey {
    fn from(value: &ModifierKeyMessage) -> Self {
        match value {
            ModifierKeyMessage::Shift => ModifierKey::Shift,
            ModifierKeyMessage::Control => ModifierKey::Control,
            ModifierKeyMessage::Alt => ModifierKey::Alt,
            ModifierKeyMessage::Command => ModifierKey::Command,
        }
    }
}

/// User clicks a mouse button over an element.
#[derive(Clone)]
pub struct ArgsClick {
    pub mouse: MouseEventArgs,
}

/// User double-clicks a mouse button over an element.
#[derive(Clone)]
pub struct ArgsDoubleClick {
    pub mouse: MouseEventArgs,
}

/// User moves the mouse while it is over an element.
#[derive(Clone)]
pub struct ArgsMouseMove {
    pub mouse: MouseEventArgs,
}

/// User scrolls the mouse wheel over an element.
#[derive(Clone)]
pub struct ArgsWheel {
    pub x: f64,
    pub y: f64,
    pub delta_x: f64,
    pub delta_y: f64,
    pub modifiers: Vec<ModifierKey>,
}

#[derive(Clone)]
pub struct ArgsCheckboxChange {
    pub checked: bool,
}

#[derive(Clone)]
pub struct ArgsTextboxChange {
    pub text: String,
}

#[derive(Clone)]
pub struct ArgsButtonClick {}

/// User presses a mouse button over an element.
#[derive(Clone)]
pub struct ArgsMouseDown {
    pub mouse: MouseEventArgs,
}

/// User releases a mouse button over an element.
#[derive(Clone)]
pub struct ArgsMouseUp {
    pub mouse: MouseEventArgs,
}

/// User moves the mouse onto an element.
#[derive(Clone)]
pub struct ArgsMouseOver {
    pub mouse: MouseEventArgs,
}

/// User moves the mouse away from an element.
#[derive(Clone)]
pub struct ArgsMouseOut {
    pub mouse: MouseEventArgs,
}

/// User right-clicks an element to open the context menu.
#[derive(Clone)]
pub struct ArgsContextMenu {
    pub mouse: MouseEventArgs,
}

/// A Size value that can be either a concrete pixel value
/// or a percent of parent bounds.

#[cfg_attr(debug_assertions, derive(Debug))]
#[derive(Copy, Clone)]
pub enum Size {
    Pixels(Numeric),
    Percent(Numeric),
    ///Pixel component, Percent component
    Combined(Numeric, Numeric),
}

impl Neg for Size {
    type Output = Size;
    fn neg(self) -> Self::Output {
        match self {
            Size::Pixels(pix) => Size::Pixels(-pix),
            Size::Percent(per) => Size::Percent(-per),
            Size::Combined(pix, per) => Size::Combined(-pix, -per),
        }
    }
}

impl Add for Size {
    type Output = Size;
    fn add(self, rhs: Self) -> Self::Output {
        let mut pixel_component: Numeric = Default::default();
        let mut percent_component: Numeric = Default::default();

        [self, rhs].iter().for_each(|size| match size {
            Size::Pixels(s) => pixel_component = pixel_component + *s,
            Size::Percent(s) => percent_component = percent_component + *s,
            Size::Combined(s0, s1) => {
                pixel_component = pixel_component + *s0;
                percent_component = percent_component + *s1;
            }
        });

        Size::Combined(pixel_component, percent_component)
    }
}

use std::ops::Sub;

impl Sub for Size {
    type Output = Size;
    fn sub(self, rhs: Self) -> Self::Output {
        let mut pixel_component: Numeric = Default::default();
        let mut percent_component: Numeric = Default::default();

        let sizes = [(self, 1), (rhs, -1)];
        for (size, multiplier) in sizes.iter() {
            match size {
                Size::Pixels(s) => {
                    pixel_component = pixel_component + *s * Numeric::from(*multiplier)
                }
                Size::Percent(s) => {
                    percent_component = percent_component + *s * Numeric::from(*multiplier)
                }
                Size::Combined(s0, s1) => {
                    pixel_component = pixel_component + *s0 * Numeric::from(*multiplier);
                    percent_component = percent_component + *s1 * Numeric::from(*multiplier);
                }
            }
        }

        Size::Combined(pixel_component, percent_component)
    }
}

impl Size {
    #[allow(non_snake_case)]
    pub fn ZERO() -> Self {
        Size::Pixels(Numeric::from(0.0))
    }

    /// Returns the wrapped percent value normalized as a float, such that 100% => 1.0.
    /// Panics if wrapped type is not a percentage.
    pub fn expect_percent(&self) -> f64 {
        match &self {
            Size::Percent(val) => val.get_as_float() / 100.0,
            _ => {
                panic!("Percentage value expected but stored value was not a percentage.")
            }
        }
    }
}

pub enum Axis {
    X,
    Y,
}

impl Size {
    //Evaluate a Size in the context of `bounds` and a target `axis`.
    //Returns a `Pixel` value as a simple f64; calculates `Percent` with respect to `bounds` & `axis`
    pub fn evaluate(&self, bounds: (f64, f64), axis: Axis) -> f64 {
        let target_bound = match axis {
            Axis::X => bounds.0,
            Axis::Y => bounds.1,
        };
        match &self {
            Size::Pixels(num) => num.get_as_float(),
            Size::Percent(num) => target_bound * (*num / 100.0),
            Size::Combined(pixel_component, percent_component) => {
                //first calc percent, then add pixel
                (target_bound * (percent_component.get_as_float() / 100.0))
                    + pixel_component.get_as_float()
            }
        }
    }
}

// Struct containing fields shared by all RenderNodes.
// Each property here is special-cased by the compiler when parsing element properties (e.g. `<SomeElement width={...} />`)
// Retrieved via <dyn InstanceNode>#get_common_properties

#[cfg_attr(debug_assertions, derive(Debug))]
#[derive(Default)]
pub struct CommonProperties {
    pub x: Option<Box<dyn PropertyInstance<Size>>>,
    pub y: Option<Box<dyn PropertyInstance<Size>>>,
    pub scale_x: Option<Box<dyn PropertyInstance<Size>>>,
    pub scale_y: Option<Box<dyn PropertyInstance<Size>>>,
    pub skew_x: Option<Box<dyn PropertyInstance<Numeric>>>,
    pub skew_y: Option<Box<dyn PropertyInstance<Numeric>>>,
    pub rotate: Option<Box<dyn PropertyInstance<Rotation>>>,
    pub anchor_x: Option<Box<dyn PropertyInstance<Size>>>,
    pub anchor_y: Option<Box<dyn PropertyInstance<Size>>>,
    pub transform: Box<dyn PropertyInstance<Transform2D>>,
    pub width: Box<dyn PropertyInstance<Size>>,
    pub height: Box<dyn PropertyInstance<Size>>,
}

impl CommonProperties {
    pub fn get_default_properties_literal() -> Vec<(String, String)> {
        Self::get_property_identifiers()
            .iter()
            .map(|id| {
                if id.0 == "transform" {
                    (
                        id.0.to_string(),
                        "Transform2D::default_wrapped()".to_string(),
                    )
                } else {
                    (id.0.to_string(), "Default::default()".to_string())
                }
            })
            .collect()
    }

    pub fn get_property_identifiers() -> Vec<(String, String)> {
        vec![
            ("x".to_string(), "Size".to_string()),
            ("y".to_string(), "Size".to_string()),
            ("scale_x".to_string(), "Size".to_string()),
            ("scale_y".to_string(), "Size".to_string()),
            ("skew_x".to_string(), "Numeric".to_string()),
            ("skew_y".to_string(), "Numeric".to_string()),
            ("anchor_x".to_string(), "Size".to_string()),
            ("anchor_y".to_string(), "Size".to_string()),
            ("rotate".to_string(), "Rotation".to_string()),
            ("transform".to_string(), "Transform2D".to_string()),
            ("width".to_string(), "Size".to_string()),
            ("height".to_string(), "Size".to_string()),
        ]
    }
}

#[cfg_attr(debug_assertions, derive(Debug))]
#[derive(Clone)]
pub enum Rotation {
    Radians(Numeric),
    Degrees(Numeric),
    Percent(Numeric),
}
impl Rotation {
    #[allow(non_snake_case)]
    pub fn ZERO() -> Self {
        Self::Radians(Numeric::from(0.0))
    }

    pub fn get_as_radians(&self) -> f64 {
        if let Self::Radians(num) = self {
            num.get_as_float()
        } else if let Self::Degrees(num) = self {
            num.get_as_float() * std::f64::consts::PI * 2.0 / 360.0
        } else if let Self::Percent(num) = self {
            num.get_as_float() * std::f64::consts::PI * 2.0 / 100.0
        } else {
            unreachable!()
        }
    }
}
impl Neg for Rotation {
    type Output = Rotation;
    fn neg(self) -> Self::Output {
        match self {
            Rotation::Degrees(deg) => Rotation::Degrees(-deg),
            Rotation::Radians(rad) => Rotation::Radians(-rad),
            Rotation::Percent(per) => Rotation::Percent(-per),
        }
    }
}
impl Into<Rotation> for Numeric {
    fn into(self) -> Rotation {
        Rotation::Radians(self)
    }
}
impl Into<Rotation> for Size {
    fn into(self) -> Rotation {
        if let Size::Percent(pix) = self {
            Rotation::Percent(pix)
        } else {
            panic!("Tried to coerce a pixel value into a rotation value; try `%` or `rad` instead of `px`.")
        }
    }
}

impl Default for Rotation {
    fn default() -> Self {
        Self::ZERO()
    }
}

impl Size {
    pub fn get_pixels(&self, parent: f64) -> f64 {
        match &self {
            Self::Pixels(p) => p.get_as_float(),
            Self::Percent(p) => parent * (p.get_as_float() / 100.0),
            Self::Combined(pix, per) => {
                (parent * (per.get_as_float() / 100.0)) + pix.get_as_float()
            }
        }
    }
}

impl Interpolatable for Size {
    fn interpolate(&self, other: &Self, t: f64) -> Self {
        match &self {
            Self::Pixels(sp) => match other {
                Self::Pixels(op) => Self::Pixels(*sp + ((*op - *sp) * Numeric::from(t))),
                Self::Percent(op) => Self::Percent(*op),
                Self::Combined(pix, per) => {
                    let pix = *sp + ((*pix - *sp) * Numeric::from(t));
                    let per = *per;
                    Self::Combined(pix, per)
                }
            },
            Self::Percent(sp) => match other {
                Self::Pixels(op) => Self::Pixels(*op),
                Self::Percent(op) => Self::Percent(*sp + ((*op - *sp) * Numeric::from(t))),
                Self::Combined(pix, per) => {
                    let pix = *pix;
                    let per = *sp + ((*per - *sp) * Numeric::from(t));
                    Self::Combined(pix, per)
                }
            },
            Self::Combined(pix, per) => match other {
                Self::Pixels(op) => {
                    let pix = *pix + ((*op - *pix) * Numeric::from(t));
                    Self::Combined(pix, *per)
                }
                Self::Percent(op) => {
                    let per = *per + ((*op - *per) * Numeric::from(t));
                    Self::Combined(*pix, per)
                }
                Self::Combined(pix0, per0) => {
                    let pix = *pix + ((*pix0 - *pix) * Numeric::from(t));
                    let per = *per + ((*per0 - *per) * Numeric::from(t));
                    Self::Combined(pix, per)
                }
            },
        }
    }
}

impl<T: Interpolatable> Interpolatable for Option<T> {
    fn interpolate(&self, other: &Self, t: f64) -> Self {
        match &self {
            Self::Some(s) => match other {
                Self::Some(o) => Some(s.interpolate(o, t)),
                _ => None,
            },
            Self::None => None,
        }
    }
}

impl Default for Size {
    fn default() -> Self {
        Self::Percent(100.0.into())
    }
}

impl From<Size> for SizePixels {
    fn from(value: Size) -> Self {
        match value {
            Size::Pixels(x) => SizePixels(x),
            _ => {
                panic!("Non-pixel Size cannot be coerced into SizePixels");
            }
        }
    }
}

#[derive(Copy, Clone)]
#[cfg_attr(debug_assertions, derive(Debug))]
pub struct SizePixels(pub Numeric);

impl Default for SizePixels {
    fn default() -> Self {
        Self(Numeric::Float(150.0))
    }
}
impl From<&SizePixels> for f64 {
    fn from(value: &SizePixels) -> Self {
        value.0.get_as_float()
    }
}

impl PartialEq<Numeric> for SizePixels {
    fn eq(&self, other: &Numeric) -> bool {
        self.0 == *other
    }
}
impl PartialEq<SizePixels> for Numeric {
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
/// Most often called as `pax_lang::log("some message")`
pub fn log(msg: &str) {
    let logging_variant = &(LOGGER.borrow().read().expect("Logger isn't registered").0);
    match logging_variant {
        PlatformSpecificLogger::Web(closure) => closure(msg),
        PlatformSpecificLogger::MacOS(closure) => {
            let msg = CString::new(msg).unwrap();
            (closure)(msg.as_ptr());
        }
    }
}

impl Mul for Size {
    type Output = Size;

    fn mul(self, rhs: Self) -> Self::Output {
        match self {
            Size::Pixels(pix0) => {
                match rhs {
                    //multiplying two pixel values adds them,
                    //in the sense of multiplying two affine translations.
                    //this might be wildly unexpected in some cases, so keep an eye on this and
                    //revisit whether to support Percent values in anchor calcs (could rescind)
                    Size::Pixels(pix1) => Size::Pixels(pix0 + pix1),
                    Size::Percent(per1) => Size::Pixels(pix0 * per1),
                    Size::Combined(pix1, per1) => Size::Pixels((pix0 * per1) + pix0 + pix1),
                }
            }
            Size::Percent(per0) => match rhs {
                Size::Pixels(pix1) => Size::Pixels(per0 * pix1),
                Size::Percent(per1) => Size::Percent(per0 * per1),
                Size::Combined(pix1, per1) => Size::Pixels((per0 * pix1) + (per0 * per1)),
            },
            Size::Combined(pix0, per0) => match rhs {
                Size::Pixels(pix1) => Size::Pixels((pix0 * per0) + pix1),
                Size::Percent(per1) => Size::Percent(pix0 * per0 * per1),
                Size::Combined(pix1, per1) => Size::Pixels((pix0 * per0) + (pix1 * per1)),
            },
        }
    }
}

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
#[cfg_attr(debug_assertions, derive(Debug))]
#[derive(Default, Clone)]
pub struct Transform2D {
    /// Keeps track of a linked list of previous Transform2Ds, assembled e.g. via multiplication
    pub previous: Option<Box<Transform2D>>,
    /// Rotation is single-dimensional for 2D rendering, representing rotation over z axis
    pub rotate: Option<Rotation>,
    pub translate: Option<[Size; 2]>,
    pub anchor: Option<[Size; 2]>,
    pub scale: Option<[Size; 2]>,
    pub skew: Option<[f64; 2]>,
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
    pub fn scale(x: Size, y: Size) -> Self {
        let mut ret = Transform2D::default();
        ret.scale = Some([x, y]);
        ret
    }
    ///Rotation over z axis
    pub fn rotate(z: Rotation) -> Self {
        let mut ret = Transform2D::default();
        ret.rotate = Some(z);
        ret
    }
    ///Translation across x-y plane, pixels
    pub fn translate(x: Size, y: Size) -> Self {
        let mut ret = Transform2D::default();
        ret.translate = Some([x, y]);
        ret
    }
    ///Describe alignment of the (0,0) position of this element as it relates to its own bounding box
    pub fn anchor(x: Size, y: Size) -> Self {
        let mut ret = Transform2D::default();
        ret.anchor = Some([x, y]);
        ret
    }

    pub fn default_wrapped() -> Box<dyn PropertyInstance<Self>> {
        Box::new(PropertyLiteral::new(Transform2D::default()))
    }
}

pub struct TransitionManager<T> {
    pub queue: VecDeque<TransitionQueueEntry<T>>,
    pub value: Option<T>,
}

#[cfg(debug_assertions)]
impl<T: std::fmt::Debug> std::fmt::Debug for TransitionManager<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TransitionManager")
            .field("queue", &self.queue)
            .field("value", &self.value)
            .finish()
    }
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

#[cfg(debug_assertions)]
impl<T: std::fmt::Debug> std::fmt::Debug for PropertyLiteral<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PropertyLiteral")
            .field("value", &self.value)
            .field("transition_manager", &self.transition_manager)
            .finish()
    }
}

impl<T> Into<Box<dyn PropertyInstance<T>>> for PropertyLiteral<T>
where
    T: Default + Clone + 'static,
{
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

    fn get_mut(&mut self) -> &mut T {
        &mut self.value
    }

    fn _get_vtable_id(&self) -> Option<usize> {
        None
    }

    fn set(&mut self, value: T) {
        self.value = value;
    }

    //FUTURE: when trait fields land in Rust, DRY this implementation vs. other <T: PropertyInstance> implementations
    fn ease_to(&mut self, new_value: T, duration_frames: u64, curve: EasingCurve) {
        self.transition_manager.value = Some(self.get().clone());
        let _ = &self.transition_manager.queue.clear();
        let _ = &self
            .transition_manager
            .queue
            .push_back(TransitionQueueEntry {
                global_frame_started: None,
                duration_frames,
                curve,
                starting_value: self.value.clone(),
                ending_value: new_value,
            });
    }

    fn ease_to_later(&mut self, new_value: T, duration_frames: u64, curve: EasingCurve) {
        if let None = self.transition_manager.value {
            //handle case where transition queue is empty -- a None value gets skipped, so populate it with Some
            self.transition_manager.value = Some(self.get().clone());
        }

        let starting_value = if self.transition_manager.queue.len() > 0 {
            self.transition_manager
                .queue
                .get(self.transition_manager.queue.len() - 1)
                .unwrap()
                .ending_value
                .clone()
        } else {
            self.value.clone()
        };

        self.transition_manager
            .queue
            .push_back(TransitionQueueEntry {
                global_frame_started: None,
                duration_frames,
                curve,
                starting_value,
                ending_value: new_value,
            });
    }

    fn _get_transition_manager(&mut self) -> Option<&mut TransitionManager<T>> {
        // log(&format!("property T not printable"));
        if let None = self.transition_manager.value {
            None
        } else {
            Some(&mut self.transition_manager)
        }
    }
}

impl<T: std::fmt::Debug> PropertyLiteral<T> {
    fn _get_transition_manager(&mut self) -> Option<&mut TransitionManager<T>> {
        // log(&format!("property: {:?}", self));
        if let None = self.transition_manager.value {
            None
        } else {
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
    #[allow(dead_code)]
    fn none(t: f64) -> f64 {
        if t == 1.0 {
            1.0
        } else {
            0.0
        }
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
        const C2: f64 = C1 * 1.525;
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
            EasingCurve::Linear => EasingEvaluators::linear(t),
            EasingCurve::InQuad => EasingEvaluators::in_quad(t),
            EasingCurve::OutQuad => EasingEvaluators::out_quad(t),
            EasingCurve::InBack => EasingEvaluators::in_back(t),
            EasingCurve::OutBack => EasingEvaluators::out_back(t),
            EasingCurve::InOutBack => EasingEvaluators::in_out_back(t),
            EasingCurve::Custom(evaluator) => (*evaluator)(t),
        };

        v0.interpolate(v1, multiplier)
    }
}

pub trait Interpolatable
where
    Self: Sized + Clone,
{
    //default implementation acts like a `None` ease — that is,
    //the first value is simply returned.
    fn interpolate(&self, _other: &Self, _t: f64) -> Self {
        self.clone()
    }
}

impl<I: Interpolatable> Interpolatable for Vec<I> {
    fn interpolate(&self, other: &Self, t: f64) -> Self {
        //FUTURE: could revisit the following assertion/constraint, perhaps with a "don't-care" approach to disjoint vec elements
        assert_eq!(
            self.len(),
            other.len(),
            "cannot interpolate between vecs of different lengths"
        );

        self.iter()
            .enumerate()
            .map(|(i, elem)| elem.interpolate(other.get(i).unwrap(), t))
            .collect()
    }
}

impl Interpolatable for f64 {
    fn interpolate(&self, other: &f64, t: f64) -> f64 {
        self + (*other - self) * t
    }
}

impl Interpolatable for bool {
    fn interpolate(&self, _other: &bool, _t: f64) -> bool {
        *self
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

impl Interpolatable for i64 {
    fn interpolate(&self, other: &i64, t: f64) -> i64 {
        (*self as f64 + (*other - self) as f64 * t) as i64
    }
}

impl Interpolatable for u64 {
    fn interpolate(&self, other: &u64, t: f64) -> u64 {
        (*self as f64 + (*other - self) as f64 * t) as u64
    }
}

impl Interpolatable for u8 {
    fn interpolate(&self, other: &u8, t: f64) -> u8 {
        (*self as f64 + (*other - *self) as f64 * t) as u8
    }
}

impl Interpolatable for u16 {
    fn interpolate(&self, other: &u16, t: f64) -> u16 {
        (*self as f64 + (*other - *self) as f64 * t) as u16
    }
}

impl Interpolatable for u32 {
    fn interpolate(&self, other: &u32, t: f64) -> u32 {
        (*self as f64 + (*other - *self) as f64 * t) as u32
    }
}

impl Interpolatable for i8 {
    fn interpolate(&self, other: &i8, t: f64) -> i8 {
        (*self as f64 + (*other - *self) as f64 * t) as i8
    }
}

impl Interpolatable for i16 {
    fn interpolate(&self, other: &i16, t: f64) -> i16 {
        (*self as f64 + (*other - *self) as f64 * t) as i16
    }
}

impl Interpolatable for i32 {
    fn interpolate(&self, other: &i32, t: f64) -> i32 {
        (*self as f64 + (*other - *self) as f64 * t) as i32
    }
}

impl Interpolatable for String {}

pub struct Timeline {
    pub playhead_position: usize,
    pub frame_count: usize,
    pub is_playing: bool,
}

#[cfg_attr(debug_assertions, derive(Debug))]
#[derive(Clone, Copy, PartialEq)]
pub enum Layer {
    Native,
    Scroller,
    Canvas,
    DontCare,
}

/// Captures information about z-index during render node traversal
/// Used for generating chassis side rendering architecture
#[cfg_attr(debug_assertions, derive(Debug))]
#[derive(Clone)]
pub struct OcclusionLayerGen {
    canvas_index: u32,
    layer: Layer,
    #[allow(dead_code)]
    parent_scroller: Option<Vec<u32>>,
}

impl OcclusionLayerGen {
    pub fn new(scroller_id: Option<Vec<u32>>) -> Self {
        OcclusionLayerGen {
            canvas_index: 0,
            layer: Layer::Canvas,
            parent_scroller: scroller_id,
        }
    }

    pub fn get_level(&mut self) -> u32 {
        self.canvas_index
    }

    pub fn get_current_layer(&mut self) -> Layer {
        self.layer.clone()
    }
    pub fn update_z_index(&mut self, layer: Layer) {
        match layer {
            Layer::DontCare => {}
            _ => {
                if self.layer != layer {
                    if layer == Layer::Canvas || layer == Layer::Scroller {
                        self.canvas_index += 1;
                    }
                }
                self.layer = layer.clone();
            }
        }
    }

    pub fn assemble_canvas_id(scroller_id: Option<Vec<u32>>, z_index: u32) -> String {
        if let Some(id) = scroller_id {
            format!("{:?}_{}", id, z_index)
        } else {
            format!("{}", z_index)
        }
    }
}

impl Interpolatable for StringBox {}

#[cfg_attr(debug_assertions, derive(Debug))]
#[derive(Clone, Default)]
pub struct StringBox {
    pub string: String,
}

impl Add for StringBox {
    type Output = Self;

    fn add(mut self, rhs: Self) -> Self::Output {
        self.string.push_str(&rhs.string.as_str());
        self
    }
}

impl From<&str> for StringBox {
    fn from(value: &str) -> Self {
        StringBox {
            string: value.to_string(),
        }
    }
}

impl From<String> for StringBox {
    fn from(value: String) -> Self {
        StringBox { string: value }
    }
}

impl From<&String> for StringBox {
    fn from(value: &String) -> Self {
        StringBox {
            string: value.to_string(),
        }
    }
}

impl From<StringBox> for String {
    fn from(value: StringBox) -> Self {
        value.string
    }
}

pub trait RenderContext {
    fn fill(&mut self, path: BezPath, brush: &PaintBrush);
    fn stroke(&mut self, path: BezPath, brush: &PaintBrush, width: f64);
    fn save(&mut self);
    fn restore(&mut self);
    fn clip(&mut self, path: BezPath);
    fn draw_image(&mut self, image: &ImageBuf, rect: Rect);
}
