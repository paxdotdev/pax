use std::{any::Any, cell::RefCell, marker::PhantomData, rc::Rc};

use slotmap::SlotMap;

use crate::{ExpressionTable, RuntimePropertiesStackFrame};

use self::private::PropId;

pub struct PropertyTable {
    entires: RefCell<SlotMap<PropId, PropertyData>>,
}

impl std::fmt::Debug for PropertyTable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PropertyTable")
            .field(
                "prop_ids",
                &self.entires.borrow().keys().collect::<Vec<_>>(),
            )
            .finish_non_exhaustive()
    }
}

impl PropertyTable {
    pub fn new() -> Self {
        Self {
            entires: Default::default(),
        }
    }

    fn add_literal_entry<T: 'static>(&self, val: T) -> PropId {
        let mut sm = self.entires.borrow_mut();
        sm.insert(PropertyData {
            value: Box::new(val),
            subscribers: Vec::with_capacity(0),
            on_change: Vec::with_capacity(0),
            prop_type: PropType::Literal(),
        })
    }

    fn add_expr_entry(
        &self,
        stack: Rc<RuntimePropertiesStackFrame>,
        vtable: Rc<ExpressionTable>,
        vtable_id: usize,
        dependents: &[PropId],
    ) -> PropId {
        let mut sm = self.entires.borrow_mut();
        let expr_id = sm.insert(PropertyData {
            value: Box::new(()),
            subscribers: Vec::with_capacity(0),
            on_change: Vec::with_capacity(0),
            prop_type: PropType::Expr {
                dirty: true,
                vtable_id,
                subscriptions: dependents.to_vec(),
                stack,
                vtable,
            },
        });
        for id in dependents {
            self.with_prop_data(*id, |dep_prop| {
                dep_prop.subscribers.push(expr_id);
            });
        }
        expr_id
    }

    fn with_prop_data<V>(&self, id: PropId, f: impl FnOnce(&mut PropertyData) -> V) -> V {
        let mut sm = self.entires.borrow_mut();
        let prop_data = sm.get_mut(id).expect("entry exists");
        f(prop_data)
    }

    fn get_value<T: PropVal + 'static>(&self, id: PropId) -> T {
        self.with_prop_data(id, |prop_data| {
            if let PropType::Expr {
                dirty,
                vtable,
                stack,
                vtable_id,
                ..
            } = &mut prop_data.prop_type
            {
                // This dirty checking should be done automatically by sub-components (dependents)
                // of the expression during the "get" calls while computing it.
                if *dirty == true {
                    prop_data.value = vtable.compute_vtable_value(&stack, *vtable_id);
                    *dirty = false;
                }
            }
            let value = prop_data.value.downcast_ref::<T>().expect("correct type");
            value.clone()
        })
    }

    fn set_value<T: PropVal + 'static>(&self, id: PropId, value: T) {
        let mut to_dirty = vec![];
        self.with_prop_data(id, |prop_data| {
            prop_data.value = Box::new(value);
            to_dirty.extend_from_slice(&prop_data.subscribers);
        });

        while let Some(dep_id) = to_dirty.pop() {
            self.with_prop_data(dep_id, |dep_data| {
                if dep_id == id {
                    panic!("property cycle detected");
                }
                let PropType::Expr { ref mut dirty, .. } = dep_data.prop_type else {
                    panic!("non-expressions shouldn't depend on other properties")
                };
                *dirty = true;
                to_dirty.extend_from_slice(&dep_data.subscribers);
            });
        }
    }

    fn remove(&self, id: PropId) {
        // TODO make this decrease ref count
        // instead of directly removing
        let prop_data = {
            let mut sm = self.entires.borrow_mut();
            let prop_data = sm.remove(id).expect("tried to remove non-existent prop");
            prop_data
        };
        if let PropType::Expr { subscriptions, .. } = prop_data.prop_type {
            for subscription in subscriptions {
                self.with_prop_data(subscription, |sub| {
                    sub.subscribers.retain(|v| v != &id);
                });
            }
        }
    }
}

struct PropertyData {
    subscribers: Vec<PropId>,
    value: Box<dyn Any>,
    on_change: Vec<Box<dyn Fn()>>,
    prop_type: PropType,
}

enum PropType {
    Literal(),
    // expr index (in Vtable), and the id's that
    // should make this expr update
    Expr {
        //
        stack: Rc<RuntimePropertiesStackFrame>,
        vtable: Rc<ExpressionTable>,
        dirty: bool,
        vtable_id: usize,
        subscriptions: Vec<PropId>,
    },
}

pub struct Property<T> {
    id: PropId,
    table: Rc<PropertyTable>,
    _phantom: PhantomData<T>,
}

pub trait PropVal: Clone + 'static {}
impl<T: Clone + 'static> PropVal for T {}

// Don't expose these outside this module
mod private {
    slotmap::new_key_type!(
        pub struct PropId;
    );

    pub trait HasPropId {
        fn get_id(&self) -> PropId;
    }
}

impl<T> private::HasPropId for Property<T> {
    fn get_id(&self) -> PropId {
        self.id
    }
}

impl<T: PropVal> Property<T> {
    pub fn literal(table: &Rc<PropertyTable>, val: T) -> Self {
        let id = table.add_literal_entry(val);
        Self {
            id,
            table: Rc::clone(table),
            _phantom: PhantomData {},
        }
    }

    pub fn expression(
        table: &Rc<PropertyTable>,
        vtable: &Rc<ExpressionTable>,
        stack: &Rc<RuntimePropertiesStackFrame>,
        vtable_id: usize,
        dependents: &[&dyn private::HasPropId],
    ) -> Self {
        let dependent_property_ids: Vec<_> = dependents.iter().map(|v| v.get_id()).collect();
        let id = table.add_expr_entry(
            Rc::clone(stack),
            Rc::clone(vtable),
            vtable_id,
            &dependent_property_ids,
        );
        Self {
            id,
            table: Rc::clone(table),
            _phantom: PhantomData {},
        }
    }

    pub fn subscribe(&self, f: impl Fn() + 'static) {
        self.table.with_prop_data(self.id, |prop_data| {
            prop_data.on_change.push(Box::new(f));
        })
    }

    pub fn get(&self) -> T {
        self.table.get_value(self.id)
    }

    pub fn set(&self, val: T) {
        self.table.set_value(self.id, val);
    }
}

impl<T> Drop for Property<T> {
    fn drop(&mut self) {
        self.table.remove(self.id);
    }
}
