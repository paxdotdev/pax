use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt::Display;
use std::hash::Hasher;
use std::ops::{Add, Deref, Mul, Neg, Sub};
use std::time::Instant;

use crate::math::Space;
use kurbo::BezPath;
pub use pax_message::*;
pub use pax_value::numeric::Numeric;
pub use pax_value::{CoercionRules, ImplToFromPaxAny, PaxValue, ToPaxValue};
use piet::{PaintBrush, UnitPoint};
use properties::{PropertyValue, UntypedProperty};

/// Marker trait that needs to be implemented for a struct for insertion and
/// deletion in a store
/// NOTE: Stored objects need to be UNIQUE for any given stack. Do not insert
/// values with types that could potentially be used in another use case,
/// instead create a local type only used for a single purpose
pub trait Store: 'static {}

use std::cell::Cell;
use std::hash::Hash;
use std::rc::{Rc, Weak};

pub mod constants;
pub mod math;
pub mod pax_value;
pub mod properties;

pub use pax_value::functions;
pub use properties::Property;

pub use pax_value::functions::register_function;
pub use pax_value::functions::Functions;
pub use pax_value::functions::HelperFunctions;

use crate::constants::COMMON_PROPERTIES_TYPE;
pub use paste;
pub use pax_message::serde;
use serde::{Deserialize, Serialize};

pub struct TransitionQueueEntry<T> {
    pub duration_frames: u64,
    pub curve: EasingCurve,
    pub ending_value: T,
}

pub trait RenderContext {
    fn fill(&mut self, layer: usize, path: BezPath, brush: &PaintBrush);
    fn stroke(&mut self, layer: usize, path: BezPath, brush: &PaintBrush, width: f64);
    fn save(&mut self, layer: usize);
    fn restore(&mut self, layer: usize);
    fn clip(&mut self, layer: usize, path: BezPath);
    fn load_image(&mut self, path: &str, image: &[u8], width: usize, height: usize);
    fn draw_image(&mut self, layer: usize, image_path: &str, rect: kurbo::Rect);
    fn get_image_size(&mut self, image_path: &str) -> Option<(usize, usize)>;
    fn transform(&mut self, layer: usize, affine: kurbo::Affine);
    fn layers(&self) -> usize;
}

#[cfg(debug_assertions)]
impl<T> std::fmt::Debug for TransitionQueueEntry<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TransitionQueueEntry")
            .field("duration_frames", &self.duration_frames)
            // .field("ending_value", &self.ending_value)
            .finish()
    }
}

#[derive(Default, Debug, Clone, Copy)]
pub enum OS {
    Mac,
    Linux,
    Windows,
    Android,
    IPhone,
    #[default]
    Unknown,
}

impl OS {
    pub fn is_mobile(&self) -> bool {
        match self {
            OS::Android | OS::IPhone => true,
            _ => false,
        }
    }

    pub fn is_desktop(&self) -> bool {
        match self {
            OS::Mac | OS::Linux | OS::Windows => true,
            _ => false,
        }
    }
}

#[derive(Default, Debug, Clone, Copy)]
pub enum Platform {
    Web,
    Native,
    #[default]
    Unknown,
}

#[derive(Default, Debug, Clone, Copy)]
pub struct Viewport {
    pub width: f64,
    pub height: f64,
}

impl ToPaxValue for Viewport {
    fn to_pax_value(self) -> PaxValue {
        PaxValue::Object(
            vec![
                ("width".to_string(), self.width.to_pax_value()),
                ("height".to_string(), self.height.to_pax_value()),
            ]
            .into_iter()
            .collect(),
        )
    }
}

impl Interpolatable for Viewport {
    fn interpolate(&self, other: &Self, t: f64) -> Self {
        Viewport {
            width: self.width + (other.width - self.width) * t,
            height: self.height + (other.height - self.height) * t,
        }
    }
}

pub struct Window;

impl Space for Window {}

// Unified events

#[derive(Clone)]
pub struct Event<T> {
    pub args: T,
    cancelled: Rc<Cell<bool>>,
}

impl<T: Clone + 'static> ImplToFromPaxAny for Event<T> {}

impl<T> Event<T> {
    pub fn new(args: T) -> Self {
        Self {
            args,
            cancelled: Default::default(),
        }
    }

    pub fn prevent_default(&self) {
        self.cancelled.set(true);
    }

    pub fn cancelled(&self) -> bool {
        self.cancelled.get()
    }
}

impl<T> Deref for Event<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.args
    }
}

/// A Clap describes either a "click" (mousedown followed by mouseup), OR a
/// "tap" with one finger (singular fingerdown event).
/// Claps are a useful alternative to most kinds of `Click` or `Tap` events,
/// when you want the same behavior for both to be contained in one place.
#[derive(Clone)]
pub struct Clap {
    pub x: f64,
    pub y: f64,
}

/// Scroll occurs when a frame is translated vertically or horizontally
/// Can be both by touch, mouse or keyboard
/// The contained `delta_x` and `delta_y` describe the horizontal and vertical translation of
/// the frame
#[derive(Clone)]
pub struct Scroll {
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
pub struct TouchStart {
    pub touches: Vec<Touch>,
}

/// A TouchMove occurs when the user moves while touching an element.
/// The contained `touches` represent a list of touch points.
#[derive(Clone)]
pub struct TouchMove {
    pub touches: Vec<Touch>,
}

/// A TouchEnd occurs when the user stops touching an element.
/// The contained `touches` represent a list of touch points.
#[derive(Clone)]
pub struct TouchEnd {
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
pub struct KeyDown {
    pub keyboard: KeyboardEventArgs,
}

/// User is pressing a key.
#[derive(Clone)]
pub struct SelectStart {}

/// User has released a key.
#[derive(Clone)]
pub struct KeyUp {
    pub keyboard: KeyboardEventArgs,
}

/// User presses a key that displays a character (alphanumeric or symbol).
#[derive(Clone)]
pub struct KeyPress {
    pub keyboard: KeyboardEventArgs,
}

// Window/component focused
#[derive(Clone)]
pub struct Focus {}

// Mouse Events

/// Common properties in mouse events.
#[derive(Clone)]
pub struct MouseEventArgs {
    pub x: f64,
    pub y: f64,
    pub button: MouseButton,
    pub modifiers: Vec<ModifierKey>,
}

#[derive(Clone, PartialEq)]
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
pub struct Click {
    pub mouse: MouseEventArgs,
}

/// User clicks a mouse button over an element.
#[derive(Clone)]
pub struct Drop {
    pub x: f64,
    pub y: f64,
    pub name: String,
    pub mime_type: String,
    pub data: Vec<u8>,
}

/// User double-clicks a mouse button over an element.
#[derive(Clone)]
pub struct DoubleClick {
    pub mouse: MouseEventArgs,
}

/// User moves the mouse while it is over an element.
#[derive(Clone)]
pub struct MouseMove {
    pub mouse: MouseEventArgs,
}

/// User scrolls the mouse wheel over an element.
#[derive(Clone)]
pub struct Wheel {
    pub x: f64,
    pub y: f64,
    pub delta_x: f64,
    pub delta_y: f64,
    pub modifiers: Vec<ModifierKey>,
}

#[derive(Clone)]
pub struct CheckboxChange {
    pub checked: bool,
}

#[derive(Clone)]
pub struct TextInput {
    pub text: String,
}

#[derive(Clone)]
pub struct TextboxChange {
    pub text: String,
}

#[derive(Clone)]
pub struct TextboxInput {
    pub text: String,
}

#[derive(Clone)]
pub struct ButtonClick {}

/// User presses a mouse button over an element.
#[derive(Clone)]
pub struct MouseDown {
    pub mouse: MouseEventArgs,
}

/// User releases a mouse button over an element.
#[derive(Clone)]
pub struct MouseUp {
    pub mouse: MouseEventArgs,
}

/// User moves the mouse onto an element.
#[derive(Clone)]
pub struct MouseOver {}

/// User moves the mouse away from an element.
#[derive(Clone)]
pub struct MouseOut {}

/// User right-clicks an element to open the context menu.
#[derive(Clone)]
pub struct ContextMenu {
    pub mouse: MouseEventArgs,
}

/// A Size value that can be either a concrete pixel value
/// or a percent of parent bounds.

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Hash)]
#[serde(crate = "crate::serde")]
pub enum Size {
    Pixels(Numeric),
    Percent(Numeric),
    ///Pixel component, Percent component
    Combined(Numeric, Numeric),
}

impl Display for Size {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Size::Pixels(val) => write!(f, "{}px", val),
            Size::Percent(val) => write!(f, "{}%", val),
            Size::Combined(pix, per) => write!(f, "{}px + {}%", pix, per),
        }
    }
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

impl Add<Percent> for Size {
    type Output = Size;
    fn add(self, rhs: Percent) -> Self::Output {
        self + Size::Percent(rhs.0)
    }
}

impl Sub<Percent> for Size {
    type Output = Size;
    fn sub(self, rhs: Percent) -> Self::Output {
        self - Size::Percent(rhs.0)
    }
}

impl Add<Size> for Percent {
    type Output = Size;
    fn add(self, rhs: Size) -> Self::Output {
        Size::Percent(self.0) + rhs
    }
}

impl Sub<Size> for Percent {
    type Output = Size;
    fn sub(self, rhs: Size) -> Self::Output {
        Size::Percent(self.0) - rhs
    }
}

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
        Size::Pixels(Numeric::F64(0.0))
    }

    /// Returns the wrapped percent value normalized as a float, such that 100% => 1.0.
    /// Panics if wrapped type is not a percentage.
    pub fn expect_percent(&self) -> f64 {
        match &self {
            Size::Percent(val) => val.to_float() / 100.0,
            Size::Pixels(val) => {
                log::warn!("Percentage value expected but stored value was pixel.");
                val.to_float() / 100.0
            }
            Size::Combined(_, percent) => {
                log::warn!("Percentage value expected but stored value was a combination.");
                percent.to_float() / 100.0
            }
        }
    }

    /// Returns the pixel value
    /// Panics if wrapped type is not pixels.
    pub fn expect_pixels(&self) -> Numeric {
        match &self {
            Size::Pixels(val) => val.clone(),
            Size::Percent(val) => {
                log::warn!("Pixel value expected but stored value was percentage.");
                val.clone()
            }
            Size::Combined(pixels, _) => {
                log::warn!("Pixel value expected but stored value was a combination.");
                pixels.clone()
            }
        }
    }
}

#[derive(Clone, Copy)]
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
            Size::Pixels(num) => num.to_float(),
            Size::Percent(num) => target_bound * (num.to_float() / 100.0),
            Size::Combined(pixel_component, percent_component) => {
                //first calc percent, then add pixel
                (target_bound * (percent_component.to_float() / 100.0)) + pixel_component.to_float()
            }
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct CommonProperty {
    name: String,
    property_type: String,
    optional: bool,
}

// Struct containing fields shared by all RenderNodes.
// Each property here is special-cased by the compiler when parsing element properties (e.g. `<SomeElement width={...} />`)
// Retrieved via <dyn InstanceNode>#get_common_properties

#[derive(Debug, Default, Clone)]
pub struct CommonProperties {
    pub id: Property<Option<String>>,
    pub x: Property<Option<Size>>,
    pub y: Property<Option<Size>>,
    pub width: Property<Option<Size>>,
    pub height: Property<Option<Size>>,
    pub anchor_x: Property<Option<Size>>,
    pub anchor_y: Property<Option<Size>>,
    //TODO change scale to Percent (can't be px)
    pub scale_x: Property<Option<Size>>,
    pub scale_y: Property<Option<Size>>,
    pub skew_x: Property<Option<Rotation>>,
    pub skew_y: Property<Option<Rotation>>,
    pub rotate: Property<Option<Rotation>>,
    pub transform: Property<Option<Transform2D>>,
    pub unclippable: Property<Option<bool>>,
    pub _raycastable: Property<Option<bool>>,
    pub _suspended: Property<Option<bool>>,
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
        COMMON_PROPERTIES_TYPE
            .iter()
            .map(|(c, t)| (c.to_string(), t.to_string()))
            .collect()
    }

    pub fn get_as_common_property() -> Vec<CommonProperty> {
        Self::get_property_identifiers()
            .iter()
            .map(|id| CommonProperty {
                name: id.0.to_string(),
                property_type: id.1.to_string(),
                optional: (id.0 == "transform" || id.0 == "width" || id.0 == "height"),
            })
            .collect()
    }

    pub fn retrieve_property_scope(&self) -> HashMap<String, Variable> {
        let CommonProperties {
            id,
            x,
            y,
            width,
            height,
            anchor_x,
            anchor_y,
            scale_x,
            scale_y,
            skew_x,
            skew_y,
            rotate,
            transform,
            unclippable,
            _raycastable,
            _suspended,
            // NOTE: remember to add an entry to the hashmap bellow as well
        } = self;

        HashMap::from([
            (
                "id".to_string(),
                Variable::new_from_typed_property(id.clone()),
            ),
            (
                "x".to_string(),
                Variable::new_from_typed_property(x.clone()),
            ),
            (
                "y".to_string(),
                Variable::new_from_typed_property(y.clone()),
            ),
            (
                "scale_x".to_string(),
                Variable::new_from_typed_property(scale_x.clone()),
            ),
            (
                "scale_y".to_string(),
                Variable::new_from_typed_property(scale_y.clone()),
            ),
            (
                "skew_x".to_string(),
                Variable::new_from_typed_property(skew_x.clone()),
            ),
            (
                "skew_y".to_string(),
                Variable::new_from_typed_property(skew_y.clone()),
            ),
            (
                "rotate".to_string(),
                Variable::new_from_typed_property(rotate.clone()),
            ),
            (
                "anchor_x".to_string(),
                Variable::new_from_typed_property(anchor_x.clone()),
            ),
            (
                "anchor_y".to_string(),
                Variable::new_from_typed_property(anchor_y.clone()),
            ),
            (
                "transform".to_string(),
                Variable::new_from_typed_property(transform.clone()),
            ),
            (
                "width".to_string(),
                Variable::new_from_typed_property(width.clone()),
            ),
            (
                "height".to_string(),
                Variable::new_from_typed_property(height.clone()),
            ),
            (
                "unclippable".to_string(),
                Variable::new_from_typed_property(unclippable.clone()),
            ),
            (
                "_raycastable".to_string(),
                Variable::new_from_typed_property(_raycastable.clone()),
            ),
            (
                "_suspended".to_string(),
                Variable::new_from_typed_property(_suspended.clone()),
            ),
        ])
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

pub struct TransitionManager<T> {
    queue: VecDeque<TransitionQueueEntry<T>>,
    /// The value we are currently transitioning from
    transition_checkpoint_value: T,
    /// The time the current transition started
    origin_frames_elapsed: u64,
}

#[cfg(debug_assertions)]
impl<T> std::fmt::Debug for TransitionManager<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TransitionManager")
            .field("queue", &self.queue)
            // .field("value", &self.transition_checkpoint_value)
            .finish()
    }
}

impl<T: Interpolatable> TransitionManager<T> {
    pub fn new(value: T, current_time: u64) -> Self {
        Self {
            queue: VecDeque::new(),
            transition_checkpoint_value: value,
            origin_frames_elapsed: current_time,
        }
    }

    pub fn push_transition(&mut self, transition: TransitionQueueEntry<T>) {
        self.queue.push_back(transition);
    }

    pub fn reset_transitions(&mut self, current_time: u64) {
        // update current value as to ease from this position
        let eased_value = self.compute_eased_value(current_time);
        self.transition_checkpoint_value =
            eased_value.unwrap_or(self.transition_checkpoint_value.clone());
        self.queue.clear();
        self.origin_frames_elapsed = current_time;
    }

    pub fn compute_eased_value(&mut self, frames_elapsed: u64) -> Option<T> {
        let global_fe = frames_elapsed;
        let origin_fe = &mut self.origin_frames_elapsed;

        // Fast-forward transitions that have already passed
        while global_fe - *origin_fe > self.queue.front()?.duration_frames {
            let curr = self.queue.pop_front()?;
            *origin_fe += curr.duration_frames;
            self.transition_checkpoint_value = curr.ending_value;
        }
        let current_transition = self.queue.front()?;
        let local_fe = global_fe - *origin_fe;
        let progress = local_fe as f64 / current_transition.duration_frames as f64;
        let interpolated_val = current_transition.curve.interpolate(
            &self.transition_checkpoint_value,
            &current_transition.ending_value,
            progress,
        );
        Some(interpolated_val)
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

impl<I: Clone + 'static> ImplToFromPaxAny for std::ops::Range<I> {}
impl<T: 'static> ImplToFromPaxAny for Rc<T> {}
impl<T: Clone + 'static> ImplToFromPaxAny for Weak<T> {}
impl<T: Clone + 'static> ImplToFromPaxAny for Option<T> {}

impl<T1: Clone + 'static, T2: Clone + 'static> ImplToFromPaxAny for (T1, T2) {}

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

impl<I: Interpolatable> Interpolatable for std::ops::Range<I> {
    fn interpolate(&self, _other: &Self, _t: f64) -> Self {
        self.start.interpolate(&_other.start, _t)..self.end.interpolate(&_other.end, _t)
    }
}
impl Interpolatable for () {}

impl<T: ?Sized + Clone> Interpolatable for HashSet<T> {}
impl<T: ?Sized + Clone> Interpolatable for VecDeque<T> {}
impl<T: ?Sized> Interpolatable for Rc<T> {}
impl<T: Interpolatable> Interpolatable for Weak<T> {}
impl<T1: Interpolatable, T2: Interpolatable> Interpolatable for (T1, T2) {}

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

impl Interpolatable for kurbo::BezPath {}

impl Interpolatable for Instant {}

impl Interpolatable for char {}

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

impl Interpolatable for i128 {
    fn interpolate(&self, other: &i128, t: f64) -> i128 {
        (*self as f64 + (*other - self) as f64 * t) as i128
    }
}

impl Interpolatable for u128 {
    fn interpolate(&self, other: &u128, t: f64) -> u128 {
        (*self as f64 + (*other - self) as f64 * t) as u128
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Layer {
    Native,
    Canvas,
    DontCare,
}

/// Raw Percent type, which we use for serialization and dynamic traversal.  At the time
/// of authoring, this type is not used directly at runtime, but is intended for `into` coercion
/// into downstream types, e.g. ColorChannel, Rotation, and Size.  This allows us to be "dumb"
/// about how we parse `%`, and allow the context in which it is used to pull forward a specific
/// type through `into` inference.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Percent(pub Numeric);

impl Display for Percent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}%", self.0)
    }
}

impl Interpolatable for Percent {
    fn interpolate(&self, other: &Self, t: f64) -> Self {
        Self(self.0.interpolate(&other.0, t))
    }
}

impl From<f64> for ColorChannel {
    fn from(value: f64) -> Self {
        Numeric::F64(value).into()
    }
}

impl From<i32> for ColorChannel {
    fn from(value: i32) -> Self {
        Numeric::from(value).into()
    }
}

impl Into<ColorChannel> for Percent {
    fn into(self) -> ColorChannel {
        ColorChannel::Percent(self.0)
    }
}

impl Into<Size> for Percent {
    fn into(self) -> Size {
        Size::Percent(self.0)
    }
}

impl Into<Rotation> for Percent {
    fn into(self) -> Rotation {
        Rotation::Percent(self.0)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ColorChannel {
    Rotation(Rotation),
    /// [0,255]
    Integer(u8),
    /// [0.0, 100.0]
    Percent(Numeric),
}

impl Display for ColorChannel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ColorChannel::Rotation(rot) => write!(f, "{}", rot),
            ColorChannel::Integer(int) => write!(f, "{}", int),
            ColorChannel::Percent(per) => write!(f, "{}%", per),
        }
    }
}

impl Default for ColorChannel {
    fn default() -> Self {
        Self::Percent(Numeric::F64(50.0))
    }
}

impl From<Numeric> for Rotation {
    fn from(value: Numeric) -> Self {
        Rotation::Degrees(value)
    }
}

impl From<Numeric> for ColorChannel {
    fn from(value: Numeric) -> Self {
        Self::Integer(value.to_int().clamp(0, 255) as u8)
    }
}

impl ColorChannel {
    ///Normalizes this ColorChannel as a float [0.0, 1.0]
    pub fn to_float_0_1(&self) -> f64 {
        match self {
            Self::Percent(per) => (per.to_float() / 100.0).clamp(0_f64, 1_f64),
            Self::Integer(zero_to_255) => {
                let f_zero = (*zero_to_255) as f64;
                (f_zero / 255.0_f64).clamp(0_f64, 1_f64)
            }
            Self::Rotation(rot) => rot.to_float_0_1(),
        }
    }
}

#[allow(non_camel_case_types)]
#[derive(Default, Clone, Serialize, Deserialize, Debug, PartialEq)]
pub enum Color {
    /// Models a color in the RGB space, with an alpha channel of 100%
    rgb(ColorChannel, ColorChannel, ColorChannel),
    /// Models a color in the RGBA space
    rgba(ColorChannel, ColorChannel, ColorChannel, ColorChannel),

    /// Models a color in the HSL space.
    hsl(Rotation, ColorChannel, ColorChannel),
    /// Models a color in the HSLA space.
    hsla(Rotation, ColorChannel, ColorChannel, ColorChannel),

    #[default]
    SLATE,
    GRAY,
    ZINC,
    NEUTRAL,
    STONE,
    RED,
    ORANGE,
    AMBER,
    YELLOW,
    LIME,
    GREEN,
    EMERALD,
    TEAL,
    CYAN,
    SKY,
    BLUE,
    INDIGO,
    VIOLET,
    PURPLE,
    FUCHSIA,
    PINK,
    ROSE,
    BLACK,
    WHITE,
    TRANSPARENT,
    NONE,
}

impl Hash for Color {
    fn hash<H: Hasher>(&self, state: &mut H) {
        core::mem::discriminant(self).hash(state);
    }
}

// implement display respecting all color enum variants, printing out their names
impl Display for Color {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::rgb(r, g, b) => write!(f, "rgb({}, {}, {})", r, g, b),
            Self::rgba(r, g, b, a) => write!(f, "rgba({}, {}, {}, {})", r, g, b, a),
            Self::hsl(h, s, l) => write!(f, "hsl({}, {}, {})", h, s, l),
            Self::hsla(h, s, l, a) => write!(f, "hsla({}, {}, {}, {})", h, s, l, a),
            Self::SLATE => write!(f, "SLATE"),
            Self::GRAY => write!(f, "GRAY"),
            Self::ZINC => write!(f, "ZINC"),
            Self::NEUTRAL => write!(f, "NEUTRAL"),
            Self::STONE => write!(f, "STONE"),
            Self::RED => write!(f, "RED"),
            Self::ORANGE => write!(f, "ORANGE"),
            Self::AMBER => write!(f, "AMBER"),
            Self::YELLOW => write!(f, "YELLOW"),
            Self::LIME => write!(f, "LIME"),
            Self::GREEN => write!(f, "GREEN"),
            Self::EMERALD => write!(f, "EMERALD"),
            Self::TEAL => write!(f, "TEAL"),
            Self::CYAN => write!(f, "CYAN"),
            Self::SKY => write!(f, "SKY"),
            Self::BLUE => write!(f, "BLUE"),
            Self::INDIGO => write!(f, "INDIGO"),
            Self::VIOLET => write!(f, "VIOLET"),
            Self::PURPLE => write!(f, "PURPLE"),
            Self::FUCHSIA => write!(f, "FUCHSIA"),
            Self::PINK => write!(f, "PINK"),
            Self::ROSE => write!(f, "ROSE"),
            Self::BLACK => write!(f, "BLACK"),
            Self::WHITE => write!(f, "WHITE"),
            Self::TRANSPARENT => write!(f, "TRANSPARENT"),
            Self::NONE => write!(f, "NONE"),
        }
    }
}

impl Color {
    //TODO: build out tint api and consider other color transforms
    //pub fn tint(tint_offset_amount) -> Self {...}

    pub fn rgb(r: ColorChannel, g: ColorChannel, b: ColorChannel) -> Self {
        Self::rgb(r, g, b)
    }

    pub fn rgba(r: ColorChannel, g: ColorChannel, b: ColorChannel, a: ColorChannel) -> Self {
        Self::rgba(r, g, b, a)
    }

    pub fn hsl(h: Rotation, s: ColorChannel, l: ColorChannel) -> Self {
        Self::hsl(h, s, l)
    }

    pub fn hsla(h: Rotation, s: ColorChannel, l: ColorChannel, a: ColorChannel) -> Self {
        Self::hsla(h, s, l, a)
    }

    pub fn to_piet_color(&self) -> piet::Color {
        let rgba = self.to_rgba_0_1();
        piet::Color::rgba(rgba[0], rgba[1], rgba[2], rgba[3])
    }

    pub fn from_rgba_0_1(rgba_0_1: [f64; 4]) -> Self {
        Self::rgba(
            ColorChannel::Percent(Numeric::F64(rgba_0_1[0] * 100.0)),
            ColorChannel::Percent(Numeric::F64(rgba_0_1[1] * 100.0)),
            ColorChannel::Percent(Numeric::F64(rgba_0_1[2] * 100.0)),
            ColorChannel::Percent(Numeric::F64(rgba_0_1[3] * 100.0)),
        )
    }

    pub fn from_hex(hex: &str) -> Self {
        let r = u8::from_str_radix(&hex[0..2], 16).unwrap() as f64 / 255.0;
        let g = u8::from_str_radix(&hex[2..4], 16).unwrap() as f64 / 255.0;
        let b = u8::from_str_radix(&hex[4..6], 16).unwrap() as f64 / 255.0;
        let a = if hex.len() == 8 {
            u8::from_str_radix(&hex[6..8], 16).unwrap() as f64 / 255.0
        } else {
            1.0
        };
        Self::rgba(
            ColorChannel::Percent(Numeric::F64(r * 100.0)),
            ColorChannel::Percent(Numeric::F64(g * 100.0)),
            ColorChannel::Percent(Numeric::F64(b * 100.0)),
            ColorChannel::Percent(Numeric::F64(a * 100.0)),
        )
    }

    // Returns a slice of four channels, normalized to [0,1]
    pub fn to_rgba_0_1(&self) -> [f64; 4] {
        match self {
            Self::hsla(h, s, l, a) => {
                let rgb = hsl_to_rgb(h.to_float_0_1(), s.to_float_0_1(), l.to_float_0_1());
                [rgb[0], rgb[1], rgb[2], a.to_float_0_1()]
            }
            Self::hsl(h, s, l) => {
                let rgb = hsl_to_rgb(h.to_float_0_1(), s.to_float_0_1(), l.to_float_0_1());
                [rgb[0], rgb[1], rgb[2], 1.0]
            }
            Self::rgba(r, g, b, a) => [
                r.to_float_0_1(),
                g.to_float_0_1(),
                b.to_float_0_1(),
                a.to_float_0_1(),
            ],
            Self::rgb(r, g, b) => [r.to_float_0_1(), g.to_float_0_1(), b.to_float_0_1(), 1.0],

            //Color constants from TailwindCSS
            Self::SLATE => Self::rgb(
                Numeric::from(0x64).into(),
                Numeric::from(0x74).into(),
                Numeric::from(0x8b).into(),
            )
            .to_rgba_0_1(),
            Self::GRAY => Self::rgb(
                Numeric::from(0x6b).into(),
                Numeric::from(0x72).into(),
                Numeric::from(0x80).into(),
            )
            .to_rgba_0_1(),
            Self::ZINC => Self::rgb(
                Numeric::from(0x71).into(),
                Numeric::from(0x71).into(),
                Numeric::from(0x7a).into(),
            )
            .to_rgba_0_1(),
            Self::NEUTRAL => Self::rgb(
                Numeric::from(0x73).into(),
                Numeric::from(0x73).into(),
                Numeric::from(0x73).into(),
            )
            .to_rgba_0_1(),
            Self::STONE => Self::rgb(
                Numeric::from(0x78).into(),
                Numeric::from(0x71).into(),
                Numeric::from(0x6c).into(),
            )
            .to_rgba_0_1(),
            Self::RED => Self::rgb(
                Numeric::from(0xeF).into(),
                Numeric::from(0x44).into(),
                Numeric::from(0x44).into(),
            )
            .to_rgba_0_1(),
            Self::ORANGE => Self::rgb(
                Numeric::from(0xf9).into(),
                Numeric::from(0x73).into(),
                Numeric::from(0x16).into(),
            )
            .to_rgba_0_1(),
            Self::AMBER => Self::rgb(
                Numeric::from(0xf5).into(),
                Numeric::from(0x9e).into(),
                Numeric::from(0x0b).into(),
            )
            .to_rgba_0_1(),
            Self::YELLOW => Self::rgb(
                Numeric::from(0xea).into(),
                Numeric::from(0xb3).into(),
                Numeric::from(0x08).into(),
            )
            .to_rgba_0_1(),
            Self::LIME => Self::rgb(
                Numeric::from(0x84).into(),
                Numeric::from(0xcc).into(),
                Numeric::from(0x16).into(),
            )
            .to_rgba_0_1(),
            Self::GREEN => Self::rgb(
                Numeric::from(0x22).into(),
                Numeric::from(0xc5).into(),
                Numeric::from(0x5e).into(),
            )
            .to_rgba_0_1(),
            Self::EMERALD => Self::rgb(
                Numeric::from(0x10).into(),
                Numeric::from(0xb9).into(),
                Numeric::from(0x81).into(),
            )
            .to_rgba_0_1(),
            Self::TEAL => Self::rgb(
                Numeric::from(0x14).into(),
                Numeric::from(0xb8).into(),
                Numeric::from(0xa6).into(),
            )
            .to_rgba_0_1(),
            Self::CYAN => Self::rgb(
                Numeric::from(0x06).into(),
                Numeric::from(0xb6).into(),
                Numeric::from(0xd4).into(),
            )
            .to_rgba_0_1(),
            Self::SKY => Self::rgb(
                Numeric::from(0x0e).into(),
                Numeric::from(0xa5).into(),
                Numeric::from(0xe9).into(),
            )
            .to_rgba_0_1(),
            Self::BLUE => Self::rgb(
                Numeric::from(0x3b).into(),
                Numeric::from(0x82).into(),
                Numeric::from(0xf6).into(),
            )
            .to_rgba_0_1(),
            Self::INDIGO => Self::rgb(
                Numeric::from(0x63).into(),
                Numeric::from(0x66).into(),
                Numeric::from(0xf1).into(),
            )
            .to_rgba_0_1(),
            Self::VIOLET => Self::rgb(
                Numeric::from(0x8b).into(),
                Numeric::from(0x5c).into(),
                Numeric::from(0xf6).into(),
            )
            .to_rgba_0_1(),
            Self::PURPLE => Self::rgb(
                Numeric::from(0xa8).into(),
                Numeric::from(0x55).into(),
                Numeric::from(0xf7).into(),
            )
            .to_rgba_0_1(),
            Self::FUCHSIA => Self::rgb(
                Numeric::from(0xd9).into(),
                Numeric::from(0x46).into(),
                Numeric::from(0xef).into(),
            )
            .to_rgba_0_1(),
            Self::PINK => Self::rgb(
                Numeric::from(0xec).into(),
                Numeric::from(0x48).into(),
                Numeric::from(0x99).into(),
            )
            .to_rgba_0_1(),
            Self::ROSE => Self::rgb(
                Numeric::from(0xf4).into(),
                Numeric::from(0x3f).into(),
                Numeric::from(0x5e).into(),
            )
            .to_rgba_0_1(),
            Self::BLACK => Self::rgb(
                Numeric::from(0x00).into(),
                Numeric::from(0x00).into(),
                Numeric::from(0x00).into(),
            )
            .to_rgba_0_1(),
            Self::WHITE => Self::rgb(
                Numeric::from(0xff).into(),
                Numeric::from(0xff).into(),
                Numeric::from(0xff).into(),
            )
            .to_rgba_0_1(),
            Self::TRANSPARENT | Self::NONE => Self::rgba(
                Numeric::from(0xff).into(),
                Numeric::from(0xff).into(),
                Numeric::from(0xFF).into(),
                Numeric::from(0x00).into(),
            )
            .to_rgba_0_1(),
        }
    }

    //Credit: Claude 3.5 Sonnet
    pub fn to_hsla_0_1(&self) -> [f64; 4] {
        let [r, g, b, a] = self.to_rgba_0_1();

        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let chroma = max - min;

        let h = if chroma == 0.0 {
            0.0
        } else if max == r {
            ((g - b) / chroma + 6.0) % 6.0 / 6.0
        } else if max == g {
            ((b - r) / chroma + 2.0) / 6.0
        } else {
            ((r - g) / chroma + 4.0) / 6.0
        };

        let l = (max + min) / 2.0;

        let s = if l == 0.0 || l == 1.0 {
            0.0
        } else {
            (max - l) / l.min(1.0 - l)
        };

        [h, s, l, a]
    }
}

//hsl_to_rgb logic borrowed & modified from https://github.com/emgyrz/colorsys.rs, licensed MIT Copyright (c) 2019 mz <emgyrz@gmail.com>
const RGB_UNIT_MAX: f64 = 255.0;
fn hsl_to_rgb(h: f64, s: f64, l: f64) -> [f64; 3] {
    if s == 0.0 {
        let unit = RGB_UNIT_MAX * l;
        return [
            unit / RGB_UNIT_MAX,
            unit / RGB_UNIT_MAX,
            unit / RGB_UNIT_MAX,
        ];
    }

    let temp1 = if l < 0.5 {
        l * (1.0 + s)
    } else {
        l + s - l * s
    };

    let temp2 = 2.0 * l - temp1;
    let hue = h;

    let temp_r = bound(hue + (1.0 / 3.0), 1.0);
    let temp_g = bound(hue, 1.0);
    let temp_b = bound(hue - (1.0 / 3.0), 1.0);

    let r = calc_rgb_unit(temp_r, temp1, temp2);
    let g = calc_rgb_unit(temp_g, temp1, temp2);
    let b = calc_rgb_unit(temp_b, temp1, temp2);
    [r / RGB_UNIT_MAX, g / RGB_UNIT_MAX, b / RGB_UNIT_MAX]
}

fn calc_rgb_unit(unit: f64, temp1: f64, temp2: f64) -> f64 {
    let mut result = temp2;
    if 6.0 * unit < 1.0 {
        result = temp2 + (temp1 - temp2) * 6.0 * unit
    } else if 2.0 * unit < 1.0 {
        result = temp1
    } else if 3.0 * unit < 2.0 {
        result = temp2 + (temp1 - temp2) * ((2.0 / 3.0) - unit) * 6.0
    }
    result * RGB_UNIT_MAX
}

pub fn bound(r: f64, entire: f64) -> f64 {
    let mut n = r;
    loop {
        let less = n < 0.0;
        let bigger = n > entire;
        if !less && !bigger {
            break n;
        }
        if less {
            n += entire;
        } else {
            n -= entire;
        }
    }
}

impl Into<ColorMessage> for &Color {
    fn into(self) -> ColorMessage {
        let rgba = self.to_rgba_0_1();
        ColorMessage::Rgba(rgba)
    }
}
impl PartialEq<ColorMessage> for Color {
    fn eq(&self, other: &ColorMessage) -> bool {
        let self_rgba = self.to_rgba_0_1();

        match other {
            ColorMessage::Rgb(other_rgba) => {
                self_rgba[0] == other_rgba[0]
                    && self_rgba[1] == other_rgba[1]
                    && self_rgba[2] == other_rgba[2]
                    && self_rgba[3] == 1.0
            }
            ColorMessage::Rgba(other_rgba) => {
                self_rgba[0] == other_rgba[0]
                    && self_rgba[1] == other_rgba[1]
                    && self_rgba[2] == other_rgba[2]
                    && self_rgba[3] == other_rgba[3]
            }
        }
    }
}
impl Interpolatable for Color {
    fn interpolate(&self, other: &Self, t: f64) -> Self {
        let rgba_s = self.to_rgba_0_1();
        let rgba_o = other.to_rgba_0_1();
        let rgba_i = [
            rgba_s[0].interpolate(&rgba_o[0], t),
            rgba_s[1].interpolate(&rgba_o[1], t),
            rgba_s[2].interpolate(&rgba_o[2], t),
            rgba_s[3].interpolate(&rgba_o[3], t),
        ];
        Color::from_rgba_0_1(rgba_i)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Copy)]
pub enum Rotation {
    Radians(Numeric),
    Degrees(Numeric),
    Percent(Numeric),
}

impl Display for Rotation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Rotation::Radians(rad) => write!(f, "{}rad", rad),
            Rotation::Degrees(deg) => write!(f, "{}deg", deg),
            Rotation::Percent(per) => write!(f, "{}%", per),
        }
    }
}

impl Default for Rotation {
    fn default() -> Self {
        Self::Degrees(Numeric::F64(0.0))
    }
}

impl Interpolatable for Rotation {
    fn interpolate(&self, other: &Self, t: f64) -> Self {
        Self::Percent(Numeric::F64(
            other.to_float_0_1() - self.to_float_0_1() * t / 100.0,
        ))
    }
}

impl Rotation {
    #[allow(non_snake_case)]
    pub fn ZERO() -> Self {
        Self::Degrees(Numeric::F64(0.0))
    }

    /// Returns a float proportional to `0deg : 0.0 :: 360deg :: 1.0`, in the domain 𝕗𝟞𝟜
    /// For example, 0rad maps to 0.0, 100% maps to 1.0, and 720deg maps to 2.0
    pub fn to_float_0_1(&self) -> f64 {
        match self {
            Self::Radians(rad) => rad.to_float() / (std::f64::consts::PI * 2.0),
            Self::Degrees(deg) => deg.to_float() / 360.0_f64,
            Self::Percent(per) => per.to_float(),
        }
    }

    pub fn get_as_radians(&self) -> f64 {
        if let Self::Radians(num) = self {
            num.to_float()
        } else if let Self::Degrees(num) = self {
            num.to_float() * std::f64::consts::PI * 2.0 / 360.0
        } else if let Self::Percent(num) = self {
            num.to_float() * std::f64::consts::PI * 2.0 / 100.0
        } else {
            unreachable!()
        }
    }

    pub fn get_as_degrees(&self) -> f64 {
        if let Self::Radians(num) = self {
            num.to_float() * 180.0 / std::f64::consts::PI
        } else if let Self::Degrees(num) = self {
            num.to_float()
        } else if let Self::Percent(num) = self {
            num.to_float() * 360.0 / 100.0
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

impl Add for Rotation {
    type Output = Rotation;

    fn add(self, rhs: Self) -> Self::Output {
        let self_rad = self.get_as_radians();
        let other_rad = rhs.get_as_radians();
        Rotation::Radians(Numeric::F64(self_rad + other_rad))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(crate = "crate::serde")]
pub enum PathElement {
    #[default]
    Empty,
    Point(Size, Size),
    Line,
    Quadratic(Size, Size),
    Cubic(Box<(Size, Size, Size, Size)>),
    Close,
}

impl Interpolatable for PathElement {}
impl HelperFunctions for PathElement {}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(crate = "crate::serde")]
pub struct Stroke {
    pub color: Property<Color>,
    pub width: Property<Size>,
}

impl Default for Stroke {
    fn default() -> Self {
        Self {
            color: Default::default(),
            width: Property::new(Size::Pixels(Numeric::F64(0.0))),
        }
    }
}

impl PartialEq for Stroke {
    fn eq(&self, other: &Self) -> bool {
        self.color.get() == other.color.get() && self.width.get() == other.width.get()
    }
}

impl Interpolatable for Stroke {
    fn interpolate(&self, _other: &Self, _t: f64) -> Self {
        // TODO interpolation
        self.clone()
    }
}

pub enum NavigationTarget {
    Current,
    New,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(crate = "crate::serde")]
pub enum Fill {
    Solid(Color),
    LinearGradient(LinearGradient),
    RadialGradient(RadialGradient),
}

impl Hash for Fill {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            Fill::Solid(color) => {
                state.write_u8(0);
                color.hash(state);
            }
            Fill::LinearGradient(linear) => {
                state.write_u8(1);
                linear.start.hash(state);
                linear.end.hash(state);
                linear.stops.hash(state);
            }
            Fill::RadialGradient(radial) => {
                state.write_u8(2);
                radial.start.hash(state);
                radial.end.hash(state);
                radial.radius.to_bits().hash(state);
                radial.stops.hash(state);
            }
        }
    }
}

impl Hash for Stroke {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.width.get().hash(state);
        self.color.get().hash(state);
    }
}

impl Interpolatable for Fill {
    fn interpolate(&self, _other: &Self, _t: f64) -> Self {
        // TODO interpolation
        self.clone()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Hash)]
#[serde(crate = "crate::serde")]
pub struct LinearGradient {
    pub start: (Size, Size),
    pub end: (Size, Size),
    pub stops: Vec<GradientStop>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(crate = "crate::serde")]
pub struct RadialGradient {
    pub end: (Size, Size),
    pub start: (Size, Size),
    pub radius: f64,
    pub stops: Vec<GradientStop>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Hash)]
#[serde(crate = "crate::serde")]
pub struct GradientStop {
    pub position: Size,
    pub color: Color,
}

impl GradientStop {
    pub fn get(color: Color, position: Size) -> GradientStop {
        GradientStop { position, color }
    }
}

impl Default for Fill {
    fn default() -> Self {
        Self::Solid(Color::default())
    }
}

impl Fill {
    pub fn to_unit_point((x, y): (Size, Size), (width, height): (f64, f64)) -> UnitPoint {
        let normalized_x = match x {
            Size::Pixels(val) => val.to_float() / width,
            Size::Percent(val) => val.to_float() / 100.0,
            Size::Combined(pix, per) => (pix.to_float() / width) + (per.to_float() / 100.0),
        };

        let normalized_y = match y {
            Size::Pixels(val) => val.to_float() / height,
            Size::Percent(val) => val.to_float() / 100.0,
            Size::Combined(pix, per) => (pix.to_float() / width) + (per.to_float() / 100.0),
        };
        UnitPoint::new(normalized_x, normalized_y)
    }

    pub fn to_piet_gradient_stops(stops: Vec<GradientStop>) -> Vec<piet::GradientStop> {
        let mut ret = Vec::new();
        for gradient_stop in stops {
            match gradient_stop.position {
                Size::Pixels(_) => {
                    panic!("Gradient stops must be specified in percentages");
                }
                Size::Percent(p) => {
                    ret.push(piet::GradientStop {
                        pos: (p.to_float() / 100.0) as f32,
                        color: gradient_stop.color.to_piet_color(),
                    });
                }
                Size::Combined(_, _) => {
                    panic!("Gradient stops must be specified in percentages");
                }
            }
        }
        ret
    }

    #[allow(non_snake_case)]
    pub fn linearGradient(
        start: (Size, Size),
        end: (Size, Size),
        stops: Vec<GradientStop>,
    ) -> Fill {
        Fill::LinearGradient(LinearGradient { start, end, stops })
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

impl Size {
    pub fn get_pixels(&self, parent: f64) -> f64 {
        match &self {
            Self::Pixels(p) => p.to_float(),
            Self::Percent(p) => parent * (p.to_float() / 100.0),
            Self::Combined(pix, per) => (parent * (per.to_float() / 100.0)) + pix.to_float(),
        }
    }
}

impl Interpolatable for Size {
    fn interpolate(&self, other: &Self, t: f64) -> Self {
        match &self {
            Self::Pixels(sp) => match other {
                Self::Pixels(op) => Self::Pixels(*sp + ((*op - *sp) * Numeric::F64(t))),
                Self::Percent(op) => Self::Percent(*op),
                Self::Combined(pix, per) => {
                    let pix = *sp + ((*pix - *sp) * Numeric::F64(t));
                    let per = *per;
                    Self::Combined(pix, per)
                }
            },
            Self::Percent(sp) => match other {
                Self::Pixels(op) => Self::Pixels(*op),
                Self::Percent(op) => Self::Percent(*sp + ((*op - *sp) * Numeric::F64(t))),
                Self::Combined(pix, per) => {
                    let pix = *pix;
                    let per = *sp + ((*per - *sp) * Numeric::F64(t));
                    Self::Combined(pix, per)
                }
            },
            Self::Combined(pix, per) => match other {
                Self::Pixels(op) => {
                    let pix = *pix + ((*op - *pix) * Numeric::F64(t));
                    Self::Combined(pix, *per)
                }
                Self::Percent(op) => {
                    let per = *per + ((*op - *per) * Numeric::F64(t));
                    Self::Combined(*pix, per)
                }
                Self::Combined(pix0, per0) => {
                    let pix = *pix + ((*pix0 - *pix) * Numeric::F64(t));
                    let per = *per + ((*per0 - *per) * Numeric::F64(t));
                    Self::Combined(pix, per)
                }
            },
        }
    }
}

impl Default for Size {
    fn default() -> Self {
        Self::Percent(Numeric::F64(100.0))
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
#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct Transform2D {
    /// Keeps track of a linked list of previous Transform2Ds, assembled e.g. via multiplication
    pub previous: Option<Box<Transform2D>>,
    /// Rotation is single-dimensional for 2D rendering, representing rotation over z axis
    pub rotate: Option<Rotation>,
    pub translate: Option<[Size; 2]>,
    pub anchor: Option<[Size; 2]>,
    pub scale: Option<[Size; 2]>,
    pub skew: Option<[Rotation; 2]>,
}

impl Interpolatable for Transform2D {}

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
}

#[derive(Clone)]
pub struct Variable {
    untyped_property: UntypedProperty,
    convert_to_pax_value: Rc<dyn Fn(UntypedProperty) -> PaxValue>,
}

impl Variable {
    pub fn new<T: PropertyValue + ToPaxValue>(untyped_property: UntypedProperty) -> Self {
        let closure = |untyped_property: UntypedProperty| {
            let property: Property<T> = Property::new_from_untyped(untyped_property.clone());
            property.get().to_pax_value()
        };

        Variable {
            untyped_property,
            convert_to_pax_value: Rc::new(closure),
        }
    }

    pub fn new_from_typed_property<T: PropertyValue + ToPaxValue>(property: Property<T>) -> Self {
        let untyped_property = property.untyped();
        let closure = |untyped_property: UntypedProperty| {
            let property: Property<T> = Property::new_from_untyped(untyped_property.clone());
            property.get().to_pax_value()
        };

        Variable {
            untyped_property,
            convert_to_pax_value: Rc::new(closure),
        }
    }

    pub fn get_untyped_property(&self) -> &UntypedProperty {
        &self.untyped_property
    }
    pub fn get_as_pax_value(&self) -> PaxValue {
        (self.convert_to_pax_value)(self.untyped_property.clone())
    }
}
