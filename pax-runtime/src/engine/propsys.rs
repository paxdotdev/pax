use std::{any::Any, cell::RefCell, marker::PhantomData, rc::Rc};

use slotmap::SlotMap;

use crate::{ExpressionTable, RuntimePropertiesStackFrame};

use self::private::PropId;

pub struct PropertyTable {
    entires: RefCell<SlotMap<PropId, PropertyData>>,
    creation_trace: RefCell<Option<Vec<PropId>>>,
}

impl std::fmt::Debug for PropertyTable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PropertyTable")
            .field(
                "prop_ids",
                &self
                    .entires
                    .borrow()
                    .iter()
                    .map(|(k, v)| (k, &v.subscribers))
                    .collect::<Vec<_>>(),
            )
            .finish_non_exhaustive()
    }
}

impl PropertyTable {
    pub fn new() -> Self {
        Self {
            entires: Default::default(),
            creation_trace: RefCell::new(None),
        }
    }

    fn add_literal_entry<T: 'static>(&self, val: T) -> PropId {
        let mut sm = self.entires.borrow_mut();
        let prop_id = sm.insert(PropertyData {
            value: Box::new(val),
            subscribers: Vec::with_capacity(0),
            on_change: Vec::with_capacity(0),
            prop_type: PropType::Literal(),
        });
        let mut trace = self.creation_trace.borrow_mut();
        match trace.as_mut() {
            Some(trace) => trace.push(prop_id),
            None => panic!("can't create a property outside of a PropertyScopeHandle::drop_scope"),
        }
        prop_id
    }

    fn add_expr_entry(
        &self,
        stack: Rc<RuntimePropertiesStackFrame>,
        vtable: Rc<ExpressionTable>,
        vtable_id: usize,
        dependents: &[PropId],
    ) -> PropId {
        let expr_id = {
            let mut sm = self.entires.borrow_mut();
            sm.insert(PropertyData {
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
            })
        };
        let mut trace = self.creation_trace.borrow_mut();
        match trace.as_mut() {
            Some(trace) => trace.push(expr_id),
            None => panic!("can't create a property outside of a PropertyScopeHandle::drop_scope"),
        }
        for id in dependents {
            self.with_prop_data(*id, |dep_prop| {
                dep_prop.subscribers.push(expr_id);
            });
        }
        expr_id
    }

    fn with_prop_data<V>(&self, id: PropId, f: impl FnOnce(&mut PropertyData) -> V) -> V {
        let mut sm = self.entires.borrow_mut();
        let prop_data = sm.get_mut(id).expect(
            "tried to access property entry that doesn't exist anymore,\
is the corresponding PropertyScope already removed?",
        );
        f(prop_data)
    }

    fn get_value<T: PropVal + 'static>(&self, id: PropId) -> T {
        let expr_update_args = self.with_prop_data(id, |prop_data| {
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
                    *dirty = false;
                    return Some((Rc::clone(&vtable), Rc::clone(stack), *vtable_id));
                }
            }
            None
        });

        if let Some((vtable, stack, vtable_id)) = expr_update_args {
            let new_value = vtable.compute_vtable_value(&stack, vtable_id);
            self.with_prop_data(id, |prop_data| {
                prop_data.value = new_value;
            });
        }

        self.with_prop_data(id, |prop_data| {
            let value = prop_data.value.downcast_ref::<T>().expect("correct type");
            value.clone()
        })
    }

    fn set_value<T: PropVal + 'static>(&self, id: PropId, value: T) {
        let mut to_dirty = vec![];
        let on_change_handlers = self.with_prop_data(id, |prop_data| {
            prop_data.value = Box::new(value);
            to_dirty.extend_from_slice(&prop_data.subscribers);
            prop_data.on_change.clone()
        });
        for f in &on_change_handlers {
            f()
        }

        while let Some(dep_id) = to_dirty.pop() {
            let on_change_handlers = self.with_prop_data(dep_id, |dep_data| {
                if dep_id == id {
                    panic!("property cycle detected");
                }
                let PropType::Expr { ref mut dirty, .. } = dep_data.prop_type else {
                    unreachable!("non-expressions shouldn't depend on other properties")
                };
                *dirty = true;
                println!("dirtying: {:?} while setting {:?}", dep_id, id);
                to_dirty.extend_from_slice(&dep_data.subscribers);
                dep_data.on_change.clone()
            });
            for f in &on_change_handlers {
                f()
            }
        }
    }

    fn remove_entry(&self, id: PropId) {
        let prop_data = {
            let mut sm = self.entires.borrow_mut();
            let prop_data = sm.remove(id).expect("tried to remove non-existent prop");
            prop_data
        };
        for sub in prop_data.subscribers {
            self.with_prop_data(sub, |s| {
                if let PropType::Expr { subscriptions, .. } = &mut s.prop_type {
                    subscriptions.retain(|s| s != &id);
                }
            });
        }
        if let PropType::Expr { subscriptions, .. } = prop_data.prop_type {
            for subscription in subscriptions {
                self.with_prop_data(subscription, |sub| {
                    sub.subscribers.retain(|v| v != &id);
                });
            }
        }
    }

    fn trace_creation_start(&self) -> Option<Vec<PropId>> {
        let mut trace = self.creation_trace.borrow_mut();
        std::mem::replace(&mut trace, Some(vec![]))
    }

    fn trace_creation_end(&self, mut old_values: Option<Vec<PropId>>) -> Option<Vec<PropId>> {
        let mut trace = self.creation_trace.borrow_mut();
        std::mem::swap(&mut *trace, &mut old_values);
        old_values
    }
}

struct PropertyData {
    value: Box<dyn Any>,
    subscribers: Vec<PropId>,
    on_change: Vec<Rc<dyn Fn()>>,
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

#[derive(Clone)]
pub struct Property<T> {
    id: PropId,
    ptable: Rc<PropertyTable>,
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
            ptable: Rc::clone(table),
            _phantom: PhantomData {},
        }
    }

    pub fn expression(
        ptable: &Rc<PropertyTable>,
        vtable: &Rc<ExpressionTable>,
        stack: &Rc<RuntimePropertiesStackFrame>,
        vtable_id: usize,
        dependents: &[&dyn private::HasPropId],
    ) -> Self {
        let dependent_property_ids: Vec<_> = dependents.iter().map(|v| v.get_id()).collect();
        let id = ptable.add_expr_entry(
            Rc::clone(stack),
            Rc::clone(vtable),
            vtable_id,
            &dependent_property_ids,
        );
        Self {
            id,
            ptable: Rc::clone(ptable),
            _phantom: PhantomData {},
        }
    }

    pub fn subscribe(&self, f: impl Fn() + 'static) {
        self.ptable.with_prop_data(self.id, |prop_data| {
            prop_data.on_change.push(Rc::new(f));
        })
    }

    pub fn get(&self) -> T {
        self.ptable.get_value(self.id)
    }

    pub fn set(&self, val: T) {
        self.ptable.set_value(self.id, val);
    }
}

pub struct PropertyScope {
    ptable: Rc<PropertyTable>,
    ids: Vec<PropId>,
}

impl PropertyScope {
    pub fn drop_scope<V>(ptable: &Rc<PropertyTable>, f: impl FnOnce() -> V) -> (V, Self) {
        let before = ptable.trace_creation_start();
        let res = f();
        let created_prpoperty_ids = ptable.trace_creation_end(before).expect("was started");
        (
            res,
            Self {
                ptable: Rc::clone(ptable),
                ids: created_prpoperty_ids,
            },
        )
    }

    pub fn drop_all(mut self) {
        for id in self.ids.drain(0..) {
            self.ptable.remove_entry(id);
        }
    }
}

impl Drop for PropertyScope {
    fn drop(&mut self) {
        if !self.ids.is_empty() {
            panic!("PropertyScopeHandle .drop_all() must be called manually before being dropped to clean up associated properties")
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        any::Any,
        cell::{Cell, RefCell},
        collections::HashMap,
        rc::Rc,
    };

    use crate::{
        propsys::PropertyScope, ExpressionContext, ExpressionTable, RuntimePropertiesStackFrame,
    };

    use super::{Property, PropertyTable};

    #[test]
    fn test_literal_set_get() {
        let ptable = Rc::new(PropertyTable::new());

        let (prop, handle) = PropertyScope::drop_scope(&ptable, || Property::literal(&ptable, 5));
        assert_eq!(prop.get(), 5);
        prop.set(2);
        assert_eq!(prop.get(), 2);
        handle.drop_all();
    }

    #[test]
    fn test_expression_get() {
        let ptable = Rc::new(PropertyTable::new());
        let vtable = Rc::new(ExpressionTable {
            table: HashMap::from([(
                0,
                Box::new(|_ec| Box::new(42) as Box<dyn Any>)
                    as Box<dyn Fn(ExpressionContext) -> Box<dyn Any>>,
            )]),
        });
        let stack = Rc::new(RuntimePropertiesStackFrame::new(Rc::new(RefCell::new(()))));

        let (prop, handle) = PropertyScope::drop_scope(&ptable, || {
            Property::<i32>::expression(&ptable, &vtable, &stack, 0, &[])
        });
        assert_eq!(prop.get(), 42);
        handle.drop_all();
    }

    #[test]
    fn test_expression_dependent_on_literal() {
        let ptable = Rc::new(PropertyTable::new());
        let vtable = Rc::new(ExpressionTable {
            table: HashMap::from([(
                0,
                Box::new(|ec: ExpressionContext| {
                    let scope = ec.stack_frame.peek_nth(0).expect("stack frame exists");
                    let scope = scope.borrow();
                    let val: &Property<i32> =
                        scope.downcast_ref().expect("it has a property stored");
                    Box::new(val.get() * 5) as Box<dyn Any>
                }) as Box<dyn Fn(ExpressionContext) -> Box<dyn Any>>,
            )]),
        });

        let ((prop_1, prop_2), handle) = PropertyScope::drop_scope(&ptable, || {
            let prop_1 = Property::literal(&ptable, 2);

            let stack = Rc::new(RuntimePropertiesStackFrame::new(Rc::new(RefCell::new(
                prop_1.clone(),
            ))));

            let prop_2 = Property::<i32>::expression(&ptable, &vtable, &stack, 0, &[&prop_1]);
            (prop_1, prop_2)
        });

        assert_eq!(prop_2.get(), 10);
        prop_1.set(3);
        assert_eq!(prop_2.get(), 15);
        handle.drop_all();
    }

    #[test]
    fn test_subscribe() {
        let ptable = Rc::new(PropertyTable::new());
        let (prop, handle) = PropertyScope::drop_scope(&ptable, || Property::literal(&ptable, 5));
        let triggered = Rc::new(Cell::new(false));
        let sub_run = triggered.clone();
        prop.subscribe(move || {
            sub_run.set(true);
        });
        prop.set(3);
        handle.drop_all();
        assert!(triggered.get());
    }

    #[test]
    fn test_larger_network() {
        let ptable = Rc::new(PropertyTable::new());
        let vtable = Rc::new(ExpressionTable {
            table: HashMap::from([
                (
                    0,
                    Box::new(|ec: ExpressionContext| {
                        // TODO replace these with variable name lookups later on
                        let prop_1 = ec.stack_frame.peek_nth(1).expect("stack frame exists");
                        let prop_1 = prop_1.borrow();
                        let prop_1: &Property<i32> =
                            prop_1.downcast_ref().expect("it has a property stored");
                        let prop_2 = ec.stack_frame.peek_nth(0).expect("stack frame exists");
                        let prop_2 = prop_2.borrow();
                        let prop_2: &Property<i32> =
                            prop_2.downcast_ref().expect("it has a property stored");
                        Box::new(prop_1.get() * prop_2.get()) as Box<dyn Any>
                    }) as Box<dyn Fn(ExpressionContext) -> Box<dyn Any>>,
                ),
                (
                    1,
                    Box::new(|ec: ExpressionContext| {
                        let prop_1 = ec.stack_frame.peek_nth(2).expect("stack frame exists");
                        let prop_1 = prop_1.borrow();
                        let prop_1: &Property<i32> =
                            prop_1.downcast_ref().expect("it has a property stored");
                        let prop_3 = ec.stack_frame.peek_nth(0).expect("stack frame exists");
                        let prop_3 = prop_3.borrow();
                        let prop_3: &Property<i32> =
                            prop_3.downcast_ref().expect("it has a property stored");
                        Box::new(prop_1.get() + prop_3.get()) as Box<dyn Any>
                    }) as Box<dyn Fn(ExpressionContext) -> Box<dyn Any>>,
                ),
            ]),
        });

        let ((prop_1, prop_2, prop_3, prop_4), handle) = PropertyScope::drop_scope(&ptable, || {
            let prop_1 = Property::literal(&ptable, 2);
            let prop_2 = Property::literal(&ptable, 6);

            let stack = Rc::new(RuntimePropertiesStackFrame::new(Rc::new(RefCell::new(
                prop_1.clone(),
            ))));
            let context = Rc::new(RefCell::new(prop_2.clone())) as Rc<RefCell<dyn Any>>;
            let stack = stack.push(&context);

            let prop_3 =
                Property::<i32>::expression(&ptable, &vtable, &stack, 0, &[&prop_1, &prop_2]);

            let context = Rc::new(RefCell::new(prop_3.clone())) as Rc<RefCell<dyn Any>>;
            let stack = stack.push(&context);
            let prop_4 =
                Property::<i32>::expression(&ptable, &vtable, &stack, 1, &[&prop_1, &prop_3]);
            (prop_1, prop_2, prop_3, prop_4)
        });

        let sub_value = Rc::new(Cell::new(0));
        let sub_value_cl = sub_value.clone();
        let prop_1_cl = prop_1.clone();
        let prop_3_cl = prop_3.clone();
        let prop_4_cl = prop_4.clone();

        prop_4.subscribe(move || {
            if prop_4_cl.get() > 3 {
                sub_value_cl.set(prop_1_cl.get());
            } else {
                sub_value_cl.set(prop_3_cl.get());
            }
        });
        assert_eq!(prop_4.get(), 14);
        prop_1.set(1);
        assert_eq!(prop_4.get(), 7);
        assert_eq!(sub_value.get(), 1);
        prop_2.set(2);
        assert_eq!(prop_4.get(), 3);
        assert_eq!(sub_value.get(), 2);
        handle.drop_all();
    }

    #[test]
    fn test_cleanup() {
        let ptable = Rc::new(PropertyTable::new());
        assert!(ptable.entires.borrow().is_empty());
        let (_, handle) = PropertyScope::drop_scope(&ptable, || Property::literal(&ptable, 5));
        assert_eq!(ptable.entires.borrow().len(), 1);
        handle.drop_all();
        assert!(ptable.entires.borrow().is_empty());
    }

    #[test]
    #[should_panic]
    fn test_use_property_after_scope_dropped() {
        let ptable = Rc::new(PropertyTable::new());
        assert!(ptable.entires.borrow().is_empty());
        let (prop, handle) = PropertyScope::drop_scope(&ptable, || Property::literal(&ptable, 5));
        assert_eq!(ptable.entires.borrow().len(), 1);
        handle.drop_all();
        prop.get();
    }

    #[test]
    #[should_panic]
    fn test_no_prop_creation_outside_of_scope() {
        let ptable = Rc::new(PropertyTable::new());
        Property::literal(&ptable, 5);
    }

    #[test]
    #[should_panic]
    fn test_scope_handle_not_call_drop_all() {
        let ptable = Rc::new(PropertyTable::new());
        let (_, _handle) = PropertyScope::drop_scope(&ptable, || Property::literal(&ptable, 5));
    }
}
