use std::cell::RefCell;
use std::rc::Rc;
use kurbo::Affine;

// /// An abstract Property that may be either: Literal,
// /// a dynamic runtime Expression, or a Timeline-bound value
pub trait Property<T> {
    fn get(&self) -> &T;
    fn register_id(&mut self, _rtc: &mut Box<dyn StringReceiver>);
    fn cache_value(&mut self, value: T);
}

/// A size value that can be either a concrete pixel value
/// or a percent of parent bounds.
#[derive(Copy, Clone)]
pub enum Size {
    Pixel(f64),
    Percent(f64),
}

pub trait StringReceiver {
    fn receive(&mut self, value: String);
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


#[derive(Default)]
//TODO: support multiplication, expressions
pub struct Transform { //Literal
    pub rotate: Option<Box<dyn Property<f64>>>, ///over z axis
    pub translate: Option<[Box<dyn Property<f64>>; 2]>,
    pub origin: Option<[Box<dyn Property<Size>>; 2]>,
    pub align: Option<[Box<dyn Property<f64>>; 2]>,
    pub scale: Option<[Box<dyn Property<f64>>; 2]>,
}

impl Transform {
    ///Scale coefficients (1.0 == 100%) over x-y plane
    pub fn scale(x: f64, y: f64) -> Self {
        let mut ret  = Transform::default();
        ret.scale = Some([Box::new(PropertyLiteral{value: x}), Box::new(PropertyLiteral{value: y})]);
        ret
    }
    ///Rotation over z axis
    pub fn rotate(z: f64) -> Self {
        let mut ret  = Transform::default();
        ret.rotate = Some(Box::new(PropertyLiteral{value: z}));
        ret
    }
    ///Translation across x-y plane, pixels
    pub fn translate(x: f64, y: f64) -> Self {
        let mut ret  = Transform::default();
        ret.translate = Some([Box::new(PropertyLiteral{value: x}), Box::new(PropertyLiteral{value: y})]);
        ret
    }
    ///Describe alignment within parent bounding box, as a starting point before
    /// affine transformations are applied
    pub fn align(x: f64, y: f64) -> Self {
        let mut ret  = Transform::default();
        ret.align = Some([Box::new(PropertyLiteral{value: x}), Box::new(PropertyLiteral{value: y})]);
        ret
    }
    ///Describe alignment of the (0,0) position of this element as it relates to its own bounding box
    pub fn origin(x: Size, y: Size) -> Self {
        let mut ret  = Transform::default();
        ret.origin = Some([Box::new(PropertyLiteral{value: x}), Box::new(PropertyLiteral{value: y})]);
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

    fn register_id(&mut self, _rtc: &mut Box<dyn StringReceiver>) {
        //no-op for Literal
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

    fn register_id(&mut self, rtc: &mut Box<dyn StringReceiver>) {
        (*rtc).receive(self.id.clone());
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
