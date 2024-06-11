use std::{fmt::{Display, Formatter}, sync::atomic::{AtomicU64, Ordering}};

use crate::{Property, PropertyValue};

static COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub struct PropertyId {
    id: u64,
}

impl Display for PropertyId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.id)
    }
}

impl PropertyId {
    pub fn new() -> Self {
        Self {
            id: COUNTER.fetch_add(1, Ordering::SeqCst),
        }
    }

    pub fn get_property<T:PropertyValue>(&self) -> Property<T> {
        Property::new_from_id(*self)
    }

    pub fn from_string(id: &str) -> Self {
        Self {
            id: id.parse().expect("Failed to parse property id"),
        }
    }
}