//! Typed stores owned by mounted nodes and discovered through `NodeContext`.

/// Marker trait for values published with `NodeContext::provide_store`.
///
/// The concrete Rust type is the lookup key. Each mounted owner can provide one
/// value per type; a nearer provider of that type shadows an ancestor's value.
/// Prefer a named struct or newtype for each purpose. A type alias does not
/// create a distinct key.
///
/// A store is ordinary Rust data and does not need `#[pax]`. Use `Property`
/// handles inside it for reactive state; plain fields do not become reactive.
/// The owner clears its providers at final unmount. Explicitly cloned handles
/// can outlive that registration, but do not reconnect to a later mount.
pub trait Store: 'static {}

/// Failure to register or borrow a store through a `NodeContext`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoreError {
    /// No provider of the requested type exists on this node or its logical ancestors.
    Missing { type_name: &'static str },
    /// The context is not mounted, its mount has ended, or its provider ancestry has expired.
    ExpiredScope,
    /// The selected provider is already borrowed by a `with_store` closure.
    /// Lookup does not fall through to an outer provider in this case.
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
