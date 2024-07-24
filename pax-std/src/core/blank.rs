use pax_engine::pax;
use crate::*;
/// A blank component, roughly an alias for <Group />, used in cases where
/// a dummy or placeholder is needed (e.g. within designer)
#[pax]
#[inlined(<Group/>)]
pub struct BlankComponent {}