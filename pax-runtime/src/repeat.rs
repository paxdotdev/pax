use std::any::Any;
use std::cell::RefCell;
use std::collections::HashMap;
use std::iter;
use std::rc::Rc;

use pax_runtime_api::properties::UntypedProperty;
use pax_runtime_api::Property;

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

///Contains modal _vec_ and _range_ variants, describing whether the Repeat source
///is encoded as a Vec<T> (where T is a `dyn Any` properties type) or as a Range<isize>
#[derive(Default)]
pub struct RepeatProperties {
    pub source_expression_vec: Option<Property<Vec<Rc<RefCell<dyn Any>>>>>,
    pub source_expression_range: Option<Property<std::ops::Range<isize>>>,
    pub iterator_i_symbol: Option<String>,
    pub iterator_elem_symbol: Option<String>,
}

pub struct RepeatItem {
    pub elem: Property<Option<Rc<RefCell<dyn Any>>>>,
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
        expanded_node: &Rc<ExpandedNode>,
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
        let cloned_expanded_node = Rc::clone(expanded_node);
        let cloned_self = Rc::clone(&self);
        let cloned_context = Rc::clone(context);
        let source_expression =
            expanded_node.with_properties_unwrapped(|properties: &mut RepeatProperties| {
                let source = if let Some(range) = &properties.source_expression_range {
                    let cp_range = range.clone();
                    let dep = vec![range.untyped()];
                    Property::computed(
                        move || {
                            cp_range
                                .get()
                                .map(|v| Rc::new(RefCell::new(v)) as Rc<RefCell<dyn Any>>)
                                .collect::<Vec<_>>()
                        },
                        &dep.iter().collect(),
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

        let deps = vec![source_expression.untyped()];
        expanded_node
            .children
            .replace_with(Property::computed_with_name(
                move || {
                    let template_children = cloned_self.base().get_instance_children();
                    let children_with_envs = iter::repeat(template_children)
                        .zip(source_expression.get().into_iter())
                        .enumerate()
                        .flat_map(|(i, (children, elem))| {
                            let property_i = Property::new(i);
                            let property_elem = Property::new(Some(Rc::clone(&elem)));
                            let new_repeat_item = Rc::new(RefCell::new(RepeatItem {
                                i: property_i.clone(),
                                elem: property_elem.clone(),
                            }))
                                as Rc<RefCell<dyn Any>>;

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
                &deps.iter().collect(),
                &format!("repeat_children (node id: {})", expanded_node.id_chain[0]),
            ));
    }
}
