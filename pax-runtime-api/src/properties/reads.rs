//! Bounded observation of actual local reads by conservative CPU producers.
//! The normal read path owns no collector. Captures never retain typed values.
use super::{private::PropertyId, UntypedProperty};
use std::{
    cell::{Cell, RefCell},
    collections::HashSet,
    rc::Rc,
};

struct Reads {
    parent: Option<Rc<RefCell<Reads>>>,
    ids: HashSet<PropertyId>,
    limit: usize,
    overflow: bool,
}

thread_local! {
    static ACTIVE: RefCell<Option<Rc<RefCell<Reads>>>> = const { RefCell::new(None) };
    static ENABLED: Cell<bool> = const { Cell::new(false) };
}

/// Observe the local Properties read by a bounded engine preparation scope.
///
/// Includes reads inside lazy evaluators and nested observation scopes. The
/// returned handles belong to the mutable producer; never store them in an
/// immutable visual commit. Raw shared snapshots are not local graph reads.
#[doc(hidden)]
#[must_use]
pub struct PropertyReadScope {
    reads: Rc<RefCell<Reads>>,
}

impl PropertyReadScope {
    pub fn enter(limit: usize) -> Self {
        let reads = ACTIVE.with(|active| {
            let reads = Rc::new(RefCell::new(Reads {
                parent: active.borrow().clone(),
                ids: Default::default(),
                limit,
                overflow: false,
            }));
            *active.borrow_mut() = Some(reads.clone());
            reads
        });
        ENABLED.with(|enabled| enabled.set(true));
        Self { reads }
    }

    /// Overflow records no more than `limit` identities and returns that limit.
    /// Properties retired inside the scope cannot be future dependencies.
    pub fn finish(self) -> Result<Vec<UntypedProperty>, usize> {
        let (ids, overflow, limit) = {
            let mut reads = self.reads.borrow_mut();
            (std::mem::take(&mut reads.ids), reads.overflow, reads.limit)
        };
        drop(self);
        if overflow {
            Err(limit)
        } else {
            Ok(ids
                .into_iter()
                .filter_map(UntypedProperty::try_from_id)
                .collect())
        }
    }
}

impl Drop for PropertyReadScope {
    fn drop(&mut self) {
        ACTIVE.with(|active| {
            let mut active = active.borrow_mut();
            assert!(
                active
                    .as_ref()
                    .is_some_and(|current| Rc::ptr_eq(current, &self.reads)),
                "property read scopes must close in stack order"
            );
            *active = self.reads.borrow().parent.clone();
            ENABLED.with(|enabled| enabled.set(active.is_some()));
        });
    }
}

pub(super) fn record(id: PropertyId) {
    if !ENABLED.with(Cell::get) {
        return;
    }
    let mut current = ACTIVE.with(|active| active.borrow().clone());
    while let Some(reads) = current {
        let mut reads = reads.borrow_mut();
        if !reads.ids.contains(&id) {
            if reads.ids.len() == reads.limit {
                reads.overflow = true;
            } else {
                reads.ids.insert(id);
            }
        }
        current = reads.parent.clone();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{properties::DependencyEventQueue, LocalProperty};
    #[test]
    fn actual_reads_include_lazy_and_nested_dependencies_without_watching_unrelated_state() {
        let source = LocalProperty::new(1_u32);
        let unrelated = LocalProperty::new(2_u32);
        let dependency = source.clone();
        let computed = LocalProperty::computed(move || dependency.get() + 1, &[source.untyped()]);
        let outer = PropertyReadScope::enter(4);
        let inner = PropertyReadScope::enter(4);
        assert_eq!(computed.read(|value| *value), 2);
        assert_eq!(inner.finish().unwrap().len(), 2);
        assert_eq!(source.get(), 1);
        let reads = outer.finish().unwrap();
        assert_eq!(reads.len(), 2);
        let queue = DependencyEventQueue::default();
        let _watch = queue.watch(&reads, 7);
        unrelated.set(4);
        assert!(queue.is_empty());
        source.set(3);
        assert_eq!(queue.pop(), Some(7));
    }

    #[test]
    fn overflow_and_unwind_restore_the_outer_scope() {
        let first = LocalProperty::new(1_u32);
        let second = LocalProperty::new(2_u32);
        let outer = PropertyReadScope::enter(2);
        let inner = PropertyReadScope::enter(1);
        first.get();
        first.get();
        second.get();
        assert_eq!(inner.finish().unwrap_err(), 1);
        assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _inner = PropertyReadScope::enter(0);
            panic!("interrupted recording");
        }))
        .is_err());
        first.get();
        assert_eq!(outer.finish().unwrap().len(), 2);
        let next = PropertyReadScope::enter(0);
        assert!(next.finish().unwrap().is_empty());
    }
}
