//! Layout units, axes, and common properties shared by renderable nodes.

use super::*;

/// A spatial size value that can be either a concrete pixel value like `25px`, a percent of parent bounds like `50%`,
/// or an additive/subtractive combination of the two like `(100% - 10px)`.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Hash)]
#[serde(crate = "crate::serde")]
pub enum Size {
    /// Concrete pixel length, such as `25px`.
    Pixels(Numeric),
    /// Percent length relative to the relevant parent bound, such as `50%`.
    Percent(Numeric),
    /// Additive pixel and percent components, such as `100% - 10px`.
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
    /// Returns a zero-pixel size.
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

/// Model of 2D cartesian axes, used to disambiguate calculations that depend on axis direction (e.g. width vs height)
#[derive(Clone, Copy)]
pub enum Axis {
    /// Horizontal axis.
    X,
    /// Vertical axis.
    Y,
}

/// Controls whether a node participates in parent layout measurement.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq, Hash)]
#[serde(crate = "crate::serde")]
pub enum LayoutRole {
    /// Normal layout behavior. The node contributes to parent hulls, autosize, and flow.
    #[default]
    Default,
    /// Parent-local positioning that does not contribute to parent hulls, autosize, or flow.
    ///
    /// `Breakout` remains in the normal render, hit-test, scroll, and clipping trees; it does
    /// not portal above ancestor frames, masks, or scrollers.
    Breakout,
}

impl Interpolatable for LayoutRole {}
impl HelperFunctions for LayoutRole {}

impl Size {
    /// Evaluate a Size in the context of `bounds` and a target `axis`.
    /// Returns a `Pixel` value as a simple f64; calculates `Percent` with respect to `bounds` & `axis`
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

// Compiler-facing metadata for one common property.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct CommonProperty {
    // Compiler-facing property name.
    name: String,
    // Compiler-facing type spelling.
    property_type: String,
    // Whether the property may be omitted without falling back to a concrete runtime value.
    optional: bool,
}

/// Properties shared by every renderable Pax node.
///
/// Each property here is special-cased by the compiler when parsing element properties,
/// for example `<SomeElement width={...} />`.
#[derive(Debug, Default, Clone)]
pub struct CommonProperties {
    /// Optional stable node identifier.
    pub id: Property<Option<String>>,
    /// Horizontal position.
    pub x: Property<Option<Size>>,
    /// Vertical position.
    pub y: Property<Option<Size>>,
    /// Horizontal extent.
    pub width: Property<Option<Size>>,
    /// Vertical extent.
    pub height: Property<Option<Size>>,
    /// Horizontal transform origin, relative to the node's own bounds.
    pub anchor_x: Property<Option<Size>>,
    /// Vertical transform origin, relative to the node's own bounds.
    pub anchor_y: Property<Option<Size>>,
    /// Horizontal scale coefficient.
    // TODO: change scale to Percent (can't be px).
    pub scale_x: Property<Option<Size>>,
    /// Vertical scale coefficient.
    pub scale_y: Property<Option<Size>>,
    /// Horizontal skew.
    pub skew_x: Property<Option<Rotation>>,
    /// Vertical skew.
    pub skew_y: Property<Option<Rotation>>,
    /// Rotation around the z axis.
    pub rotate: Property<Option<Rotation>>,
    /// Full composed transform.
    pub transform: Property<Option<Transform2D>>,
    /// Node opacity, applied to the node and its descendants.
    pub opacity: Property<Option<Opacity>>,
    /// Controls whether this node participates in parent layout measurement and flow.
    pub layout_role: Property<Option<LayoutRole>>,
    /// Allows a node to render outside an ancestor clipping frame.
    pub unclippable: Property<Option<bool>>,
    // Internal hit-testing override, used by generated components and tooling.
    pub _raycastable: Property<Option<bool>>,
    // Internal suspension flag used by runtime/designtime systems.
    pub _suspended: Property<Option<bool>>,
}

impl CommonProperties {
    // Returns default Rust expressions for compiler-generated common-property initialization.
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

    // Returns common-property names paired with compiler-facing type spellings.
    pub fn get_property_identifiers() -> Vec<(String, String)> {
        COMMON_PROPERTIES_TYPE
            .iter()
            .map(|(c, t)| (c.to_string(), t.to_string()))
            .collect()
    }

    // Returns common-property metadata in a form consumed by compiler codegen.
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

    // Exposes common properties as PAXEL variables for generated scopes.
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
            opacity,
            layout_role,
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
                "opacity".to_string(),
                Variable::new_from_typed_property(opacity.clone()),
            ),
            (
                "layout_role".to_string(),
                Variable::new_from_typed_property(layout_role.clone()),
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
    /// Resolves this size against a parent extent in pixels.
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
