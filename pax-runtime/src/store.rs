use crate::{api::NodeContext, ExpandedNode};
use pax_runtime_api::{use_RefCell, Store};
use std::{
    any::{type_name, Any, TypeId},
    cell::Cell,
    collections::HashMap,
    rc::Rc,
};

use_RefCell!();

pub use pax_runtime_api::store::StoreError;

type StoreEntry = Rc<RefCell<Box<dyn Any>>>;

#[derive(Clone, Copy, PartialEq)]
enum ScopeState {
    Pending,
    Mounted,
    Closed,
}

#[derive(Clone)]
pub(crate) struct NodeStores {
    entries: RefCell<HashMap<TypeId, StoreEntry>>,
    generation: Cell<u64>,
    parent_generation: Cell<Option<u64>>,
    state: Cell<ScopeState>,
}

impl NodeStores {
    pub(crate) fn new(parent: Option<&ExpandedNode>) -> Self {
        Self {
            entries: RefCell::new(HashMap::new()),
            generation: Cell::new(1),
            parent_generation: Cell::new(parent.map(|p| p.stores.generation())),
            state: Cell::new(ScopeState::Pending),
        }
    }

    pub(crate) fn generation(&self) -> u64 {
        self.generation.get()
    }
    pub(crate) fn is_closed(&self) -> bool {
        self.state.get() == ScopeState::Closed
    }
    pub(crate) fn has_providers(&self) -> bool {
        !self.entries.borrow().is_empty()
    }

    pub(crate) fn mount(&self, parent: Option<&ExpandedNode>) {
        if self.is_closed() {
            self.generation.set(
                self.generation
                    .get()
                    .checked_add(1)
                    .expect("store generation exhausted"),
            );
            self.parent_generation
                .set(parent.map(|p| p.stores.generation()));
        }
        self.state.set(ScopeState::Mounted);
    }

    pub(crate) fn close(&self) {
        self.state.set(ScopeState::Closed);
        self.generation.set(
            self.generation
                .get()
                .checked_add(1)
                .expect("store generation exhausted"),
        );
        // Drop user values after releasing the registry borrow.
        let entries = std::mem::take(&mut *self.entries.borrow_mut());
        drop(entries);
    }

    fn check(&self, generation: u64) -> Result<(), StoreError> {
        if self.generation() != generation || self.state.get() != ScopeState::Mounted {
            Err(StoreError::ExpiredScope)
        } else {
            Ok(())
        }
    }
}

impl NodeContext {
    pub fn provide_store<T: Store>(&self, store: T) -> Result<(), StoreError> {
        let node = self
            .expanded_node
            .upgrade()
            .ok_or(StoreError::ExpiredScope)?;
        node.stores.check(self.store_generation)?;
        let key = TypeId::of::<T>();
        let old = node.stores.entries.borrow().get(&key).cloned();
        if let Some(old) = &old {
            old.try_borrow_mut()
                .map_err(|_| StoreError::BorrowConflict {
                    type_name: type_name::<T>(),
                })?;
        }
        let replaced = node
            .stores
            .entries
            .borrow_mut()
            .insert(key, Rc::new(RefCell::new(Box::new(store))));
        drop(replaced);
        Ok(())
    }

    pub fn with_store<T: Store, V>(&self, f: impl FnOnce(&mut T) -> V) -> Result<V, StoreError> {
        let mut node = self
            .expanded_node
            .upgrade()
            .ok_or(StoreError::ExpiredScope)?;
        node.stores.check(self.store_generation)?;
        let key = TypeId::of::<T>();
        loop {
            let entry = node.stores.entries.borrow().get(&key).cloned();
            if let Some(entry) = entry {
                let mut value = entry
                    .try_borrow_mut()
                    .map_err(|_| StoreError::BorrowConflict {
                        type_name: type_name::<T>(),
                    })?;
                return Ok(f(value
                    .downcast_mut::<T>()
                    .expect("store key matches value type")));
            }
            let Some(generation) = node.stores.parent_generation.get() else {
                return Err(StoreError::Missing {
                    type_name: type_name::<T>(),
                });
            };
            let parent = node
                .template_parent
                .upgrade()
                .ok_or(StoreError::ExpiredScope)?;
            // Projection can flatten never-mounted control-flow nodes. Their
            // pending scopes remain transparent, but a closed generation cannot
            // reconnect a retained child to a new provider after remount.
            if parent.stores.generation() != generation || parent.stores.is_closed() {
                return Err(StoreError::ExpiredScope);
            }
            node = parent;
        }
    }
}
