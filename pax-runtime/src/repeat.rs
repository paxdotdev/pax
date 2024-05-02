use std::cell::RefCell;
use std::collections::HashMap;
use std::iter;
use std::rc::Rc;

use pax_runtime_api::pax_value::{PaxAny, PaxValue, ToFromPaxAny, ToFromPaxValue};
use pax_runtime_api::properties::UntypedProperty;
use pax_runtime_api::{ImplToFromPaxAny, Property};

use crate::api::Layer;
use crate::{
    BaseInstance, ExpandedNode, InstanceFlags, InstanceNode, InstantiationArgs, RuntimeContext,
};

/// A special "control-flow" primitive associated with the `for` statement.
/// Repeat allows for nodes to be rendered dynamically per data specified in `source_expression`.
/// That is: for a `source_expression` of length `n`, `Repeat` will render its
/// template `n` times, each with an embedded component context (`RepeatItem`)
/// with an index `i` and a pointer to that relevant datum `source_expression[i]`
pub struct RepeatInstance {
    pub base: BaseInstance,
}

impl ImplToFromPaxAny for RepeatProperties {}
///Contains modal _vec_ and _range_ variants, describing whether the Repeat source
///is encoded as a Vec<T> (where T is a `PaxValue` properties type) or as a Range<isize>
#[derive(Default)]
pub struct RepeatProperties {
    pub source_expression_vec: Option<Property<Vec<Rc<RefCell<PaxAny>>>>>,
    pub source_expression_range: Option<Property<std::ops::Range<isize>>>,
    pub iterator_i_symbol: Option<String>,
    pub iterator_elem_symbol: Option<String>,
}

impl ImplToFromPaxAny for RepeatItem {}

pub struct RepeatItem {
    pub elem: Property<Option<Rc<RefCell<PaxAny>>>>,
    pub i: Property<usize>,
}

impl InstanceNode for RepeatInstance {
    fn instantiate(args: InstantiationArgs) -> Rc<Self>
    where
        Self: Sized,
    {
        Rc::new(Self {
            base: BaseInstance::new(
                args,
                InstanceFlags {
                    invisible_to_slot: true,
                    invisible_to_raycasting: true,
                    layer: Layer::DontCare,
                    is_component: false,
                },
            ),
        })
    }

    #[cfg(debug_assertions)]
    fn resolve_debug(
        &self,
        f: &mut std::fmt::Formatter,
        _expanded_node: Option<&ExpandedNode>,
    ) -> std::fmt::Result {
        f.debug_struct("Repeat").finish()
    }

    fn base(&self) -> &BaseInstance {
        &self.base
    }

    fn update(
        self: Rc<Self>,
        _expanded_node: &Rc<ExpandedNode>,
        _context: &Rc<RefCell<RuntimeContext>>,
    ) {
    }

    fn handle_mount(
        self: Rc<Self>,
        expanded_node: &Rc<ExpandedNode>,
        context: &Rc<RefCell<RuntimeContext>>,
    ) {
        // No-op: wait with creating child-nodes until update tick, since the
        // condition has then been evaluated
        let weak_ref_self = Rc::downgrade(expanded_node);
        let cloned_self = Rc::clone(&self);
        let cloned_context = Rc::clone(context);
        let source_expression =
            expanded_node.with_properties_unwrapped(|properties: &mut RepeatProperties| {
                let source = if let Some(range) = &properties.source_expression_range {
                    let cp_range = range.clone();
                    let dep = [range.untyped()];
                    Property::computed(
                        move || {
                            cp_range
                                .get()
                                .map(|v| Rc::new(RefCell::new(v.to_pax_any())))
                                .collect::<Vec<_>>()
                        },
                        &dep,
                    )
                } else if let Some(vec) = &properties.source_expression_vec {
                    vec.clone()
                } else {
                    unreachable!("range or vec source must exist")
                };
                source
            });

        let i_symbol =
            expanded_node.with_properties_unwrapped(|properties: &mut RepeatProperties| {
                properties.iterator_i_symbol.clone()
            });
        let elem_symbol =
            expanded_node.with_properties_unwrapped(|properties: &mut RepeatProperties| {
                properties.iterator_elem_symbol.clone()
            });

        let deps = [source_expression.untyped()];

        let last_length = Rc::new(RefCell::new(0));

        expanded_node
            .children
            .replace_with(Property::computed_with_name(
                move || {
                    let Some(cloned_expanded_node) = weak_ref_self.upgrade() else {
                        panic!("ran evaluator after expanded node dropped (repeat elem)")
                    };
                    let source = source_expression.get();
                    let source_len = source.len();
                    if source_len == *last_length.borrow() {
                        return cloned_expanded_node.children.get();
                    }
                    *last_length.borrow_mut() = source_len;
                    let template_children = cloned_self.base().get_instance_children();
                    let children_with_envs = iter::repeat(template_children)
                        .take(source_len)
                        .enumerate()
                        .flat_map(|(i, children)| {
                            let property_i = Property::new(i);
                            let cp_source_expression = source_expression.clone();
                            let property_elem = Property::computed_with_name(
                                move || Some(Rc::clone(&cp_source_expression.get()[i])),
                                &[source_expression.untyped()],
                                "repeat elem",
                            );
                            let new_repeat_item = Rc::new(RefCell::new(
                                RepeatItem {
                                    i: property_i.clone(),
                                    elem: property_elem.clone(),
                                }
                                .to_pax_any(),
                            ));

                            let mut scope: HashMap<String, UntypedProperty> = HashMap::new();
                            if let Some(ref i_symbol) = i_symbol {
                                scope.insert(i_symbol.clone(), property_i.untyped());
                            }
                            if let Some(ref elem_symbol) = elem_symbol {
                                scope.insert(elem_symbol.clone(), property_elem.untyped());
                            }

                            let new_env = cloned_expanded_node
                                .stack
                                .push(scope.clone(), &new_repeat_item);
                            children
                                .borrow()
                                .clone()
                                .into_iter()
                                .zip(iter::repeat(new_env))
                        });
                    let ret =
                        cloned_expanded_node.generate_children(children_with_envs, &cloned_context);
                    ret
                },
                &deps,
                &format!("repeat_children (node id: {})", expanded_node.id.0),
            ));
    }
}
