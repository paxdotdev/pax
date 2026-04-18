//! Public runtime API types shared across Pax crates, user components, and platform backends.
//!
//! Most names are reexported at the crate root for compatibility, while their source modules
//! provide the browsing ontology used by the generated API docs.

use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt::Display;
use std::hash::{Hash, Hasher};
use std::ops::{Add, Deref, Mul, Neg, Sub};
use std::rc::{Rc, Weak};
use std::time::Instant;

use crate::constants::COMMON_PROPERTIES_TYPE;
use crate::math::Space;
use kurbo::BezPath;
pub use paste;
pub use pax_message::serde;
pub use pax_message::*;
pub use pax_value::numeric::Numeric;
pub use pax_value::{CoercionRules, ImplToFromPaxAny, PaxValue, ToPaxValue};
use piet::UnitPoint;
use properties::{PropertyValue, UntypedProperty};
use serde::{Deserialize, Serialize};

pub mod animation;
pub mod color;
pub mod constants;
pub mod cursor;
pub mod drawing;
pub mod events;
pub mod layout;
pub mod math;
pub mod pax_value;
pub mod platform;
pub mod properties;
pub mod rendering;
pub mod store;
pub mod transform;
pub mod variables;

pub use animation::*;
pub use color::*;
pub use drawing::*;
pub use events::*;
pub use layout::*;
pub use pax_value::functions;
pub use pax_value::functions::register_function;
pub use pax_value::functions::Functions;
pub use pax_value::functions::HelperFunctions;
pub use platform::*;
pub use properties::Property;
pub use rendering::*;
pub use store::*;
pub use transform::*;
pub use variables::*;
