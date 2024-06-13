use std::collections::HashMap;
use std::iter;
use std::rc::Rc;
use_RefCell!();

use pax_runtime_api::pax_value::{PaxAny, ToFromPaxAny};
use pax_runtime_api::{borrow, borrow_mut, use_RefCell, ImplToFromPaxAny, Property, PropertyId};

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

    fn update(self: Rc<Self>, _expanded_node: &Rc<ExpandedNode>, _context: &Rc<RuntimeContext>) {}

    fn handle_mount(
        self: Rc<Self>,
        expanded_node: &Rc<ExpandedNode>,
        context: &Rc<RuntimeContext>,
    ) {

        log::warn!("Repeat handle_mount");
        // No-op: wait with creating child-nodes until update tick, since the
        // condition has then been evaluated
        let weak_ref_self = Rc::downgrade(expanded_node);
        let cloned_self = Rc::clone(&self);
        let cloned_context = Rc::clone(context);
        
        let source_expression =
            expanded_node.with_properties_unwrapped(|properties: &mut RepeatProperties| {
                let source = if let Some(range) = &properties.source_expression_range {
                    let cp_range = range.clone();
                    log::warn!("range: {:?}", cp_range);
                    let dep = [range.get_id()];
                    Property::expression(
                        move || {
                            cp_range
                                .get()
                                .clone()
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

        

        let last_length = Rc::new(RefCell::new(0));
        let cloned_source_expression = source_expression.clone();
        
        let _ = source_expression.clone().subscribe(  move || {
            let Some(cloned_expanded_node) = weak_ref_self.upgrade() else {
                panic!("ran evaluator after expanded node dropped (repeat elem)")
            };
            let source = cloned_source_expression.get();
            let source_len = source.len();
            if source_len == *borrow!(last_length) {
                return;
            }
            log::warn!("source_len: {}", source_len);
            *borrow_mut!(last_length) = source_len;
            let template_children = cloned_self.base().get_instance_children();
            let children_with_envs = iter::repeat(template_children)
                .take(source_len)
                .enumerate()
                .flat_map(|(i, children)| {
                    let property_i = Property::new(i);
                    let cp_source_expression = cloned_source_expression.clone();
                    let property_elem = Property::expression(
                        move || {
                            Some(Rc::clone(&cp_source_expression.get().get(i).unwrap_or_else(|| panic!(
                                "engine error: tried to access index {} of an array source that now only contains {} elements",
                                i, cp_source_expression.get().len()
                            ))))
                        },
                        &[cloned_source_expression.get_id()],
                    );
                    let new_repeat_item = Rc::new(RefCell::new(
                        RepeatItem {
                            i: property_i.clone(),
                            elem: property_elem.clone(),
                        }
                        .to_pax_any(),
                    ));

                    let mut scope: HashMap<String, PropertyId> = HashMap::new();
                    if let Some(ref i_symbol) = i_symbol {
                        log::warn!("i_symbol: {:?}", i_symbol);
                        scope.insert(i_symbol.clone(), property_i.get_id());
                    }
                    if let Some(ref elem_symbol) = elem_symbol {
                        log::warn!("elem_symbol: {:?}", elem_symbol);
                        scope.insert(elem_symbol.clone(), property_elem.get_id());
                    }

                    let new_env = cloned_expanded_node.stack.push(scope, &new_repeat_item);
                    borrow!(children)
                        .clone()
                        .into_iter()
                        .zip(iter::repeat(new_env))
                });
            let children = cloned_expanded_node.generate_children(children_with_envs, &cloned_context);
            cloned_expanded_node.attach_children(children, &cloned_context);
        });
        source_expression.set(source_expression.get());
    }
}
