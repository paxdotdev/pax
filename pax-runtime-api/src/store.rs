//! Store marker traits for runtime-managed state.

/// Marker trait for types that can be inserted into a Pax local store.
///
/// Stored objects need to be unique for any given stack. Avoid inserting broad
/// reusable types directly; prefer a local newtype that represents one specific
/// store purpose.
pub trait Store: 'static {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoreError {
    Missing { type_name: &'static str },
    ExpiredScope,
    BorrowConflict { type_name: &'static str },
}

impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing { type_name } => write!(f, "no provider for store {type_name}"),
            Self::ExpiredScope => f.write_str("store scope is not mounted or has expired"),
            Self::BorrowConflict { type_name } => {
                write!(f, "store {type_name} is already borrowed")
            }
        }
    }
}

impl std::error::Error for StoreError {}
