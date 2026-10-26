//! An isolated descriptor loader; the existing runtime still constructs every node.
use crate::*;
use pax_engine::api::{
    pax_value::{PaxAny, ToFromPaxAny},
    Variable,
};
use pax_engine::pax_manifest::{self as manifest, PaxManifest, TypeId};
use pax_engine::pax_runtime::{self as rt, DefinitionToInstanceTraverser, InstanceNode};
use std::{
    collections::HashMap,
    rc::{Rc, Weak},
};
thread_local! {
 static BASE:RefCell<HashMap<TypeId,&'static rt::ErasedComponentDescriptor>>=RefCell::new(HashMap::new());
 static BUNDLE:RefCell<Option<Rc<Bundle>>>=const{RefCell::new(None)};
 static INSTANCES:RefCell<arena::Arena<Weak<Properties>>>=RefCell::new(Default::default());
}
struct Bundle {
    schema: wire::Schema,
    traverser: Traverser,
}
struct Traverser {
    manifest: RefCell<PaxManifest>,
    descriptors: HashMap<TypeId, &'static rt::ErasedComponentDescriptor>,
}
impl DefinitionToInstanceTraverser for Traverser {
    fn new(_: PaxManifest) -> Self {
        unreachable!()
    }
    fn get_manifest(&self) -> std::cell::Ref<'_, PaxManifest> {
        self.manifest.borrow()
    }
    fn get_component_descriptor(
        &self,
        id: &TypeId,
    ) -> Option<&'static rt::ErasedComponentDescriptor> {
        self.descriptors.get(id).copied()
    }
}
struct Properties {
    component: usize,
    fields: Vec<u32>,
    context: Cell<u32>,
    token: Cell<u32>,
}
impl std::ops::Drop for Properties {
    fn drop(&mut self) {
        let token = self.token.replace(0);
        if token != 0 {
            bridge::call(5, token, 0);
        }
        INSTANCES.with(|i| i.borrow_mut().remove(self.context.get()));
        for id in &self.fields {
            bridge::release(*id)
        }
    }
}
fn bundle() -> Rc<Bundle> {
    BUNDLE.with(|b| b.borrow().as_ref().unwrap().clone())
}
fn properties(value: &PaxAny) -> Rc<Properties> {
    match value {
        PaxAny::Any(v) => v.downcast_ref::<Rc<Properties>>().unwrap().clone(),
        _ => unreachable!(),
    }
}
fn create(index: usize) -> PaxAny {
    let bundle = bundle();
    let fields = bundle.schema.components[index]
        .fields
        .iter()
        .map(|f| bridge::add(fields::Field::new(f.kind, f.default.clone())))
        .collect();
    let props = Rc::new(Properties {
        component: index,
        fields,
        context: Cell::new(0),
        token: Cell::new(0),
    });
    props
        .context
        .set(INSTANCES.with(|i| i.borrow_mut().insert(Rc::downgrade(&props))));
    PaxAny::Any(Box::new(props))
}
pub fn field_handle(context: u32, index: u32) -> u32 {
    let props = INSTANCES.with(|i| i.borrow().get(context).upgrade().unwrap());
    props.fields[index as usize]
}
pub fn instances() -> u32 {
    INSTANCES.with(|i| i.borrow().len() as u32)
}
fn scope(_: &'static (dyn std::any::Any + Sync), value: &PaxAny) -> HashMap<String, Variable> {
    let p = properties(value);
    let b = bundle();
    b.schema.components[p.component]
        .fields
        .iter()
        .zip(&p.fields)
        .map(|(f, id)| (f.name.clone(), bridge::get(*id).borrow().variable()))
        .collect()
}
fn apply(
    d: &'static (dyn std::any::Any + Sync),
    value: &mut PaxAny,
    columns: &rt::RuntimeResolvedPropertyColumns,
    stack: &Rc<rt::RuntimePropertiesStackFrame>,
    node: Option<&rt::ExpandedNode>,
) {
    let props = properties(value);
    let b = bundle();
    for (field, id) in b.schema.components[props.component]
        .fields
        .iter()
        .zip(&props.fields)
    {
        if let Some(entries) = columns.get(&field.name) {
            let s = scope(d, value);
            let handle = bridge::get(*id);
            let mut v = handle.borrow().clone();
            v.apply(&field.name, field.default.clone(), entries, stack, node, &s);
            *handle.borrow_mut() = v;
        }
    }
}
fn mount(value: Rc<RefCell<PaxAny>>, _: &NodeContext, _: Option<PaxAny>) {
    let p = properties(&value.borrow());
    assert_eq!(p.token.get(), 0);
    p.token
        .set(bridge::call(1, p.component as u32, p.context.get()));
}
fn unmount(value: Rc<RefCell<PaxAny>>, _: &NodeContext, _: Option<PaxAny>) {
    let p = properties(&value.borrow());
    let token = p.token.replace(0);
    if token != 0 {
        bridge::call(5, token, 0);
    }
}
fn instantiate(mut args: rt::InstantiationArgs) -> Rc<dyn InstanceNode> {
    let registry = args.handler_registry.get_or_insert_with(Default::default);
    {
        let mut r = registry.borrow_mut();
        r.handlers
            .entry("mount".into())
            .or_default()
            .insert(0, rt::Handler::new_component_handler(mount));
        r.handlers
            .entry("unmount".into())
            .or_default()
            .push(rt::Handler::new_component_handler(unmount));
    }
    rt::ComponentInstance::instantiate(args)
}
fn invoke(index: usize, value: Rc<RefCell<PaxAny>>, ctx: &NodeContext, args: Option<PaxAny>) {
    let props = properties(&value.borrow());
    let b = bundle();
    let handler = b.schema.components[props.component]
        .handlers
        .iter()
        .find(|h| h.index == index)
        .unwrap();
    macro_rules! event {
        ($ty:ty) => {{
            let event = <$ty>::ref_from_pax_any(args.as_ref().unwrap()).unwrap();
            let payload = events::WireEvent::encode(&event.args);
            let cancelled = bridge::with_context(ctx, payload, || {
                bridge::call(2, props.token.get(), index as u32)
            });
            if cancelled != 0 {
                event.prevent_default()
            }
        }};
    }
    match handler.event.as_str() {
        "" => {
            assert!(args.is_none());
            bridge::with_context(ctx, serde_json::Value::Null, || {
                bridge::call(2, props.token.get(), index as u32)
            });
        }
        "MouseOver" => event!(Event<MouseOver>),
        "MouseOut" => event!(Event<MouseOut>),
        "Click" => event!(Event<Click>),
        "KeyDown" => event!(Event<KeyDown>),
        "Wheel" => event!(Event<Wheel>),
        "TouchStart" => event!(Event<TouchStart>),
        "ViewportProximityChange" => event!(Event<ViewportProximityChange>),
        "ViewportProximityExit" => event!(Event<ViewportProximityExit>),
        _ => panic!("unsupported event"),
    }
}
include!("trampolines.rs");
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn probe_stage(definition: &[u8], schema: &str) -> Result<(), wasm_bindgen::JsValue> {
    stage(definition, schema).map_err(|e| wasm_bindgen::JsValue::from_str(&e))
}
fn stage(definition: &[u8], schema: &str) -> Result<(), String> {
    if BUNDLE.with(|b| b.borrow().is_some()) {
        return Err("already staged".into());
    }
    let mut manifest = manifest::binary::from_slice(definition).map_err(|e| e.to_string())?;
    let schema: wire::Schema = serde_json::from_str(schema).map_err(|e| e.to_string())?;
    if schema.abi != 3 || schema.components.len() > CREATORS.len() {
        return Err("unsupported ABI or capacity".into());
    }
    let base = crate::init_manifest_for("crate::Shell");
    let mut descriptors = BASE.with(|b| b.borrow().clone());
    for (i, c) in schema.components.iter().enumerate() {
        let id = manifest
            .components
            .keys()
            .find(|id| id.get_unique_identifier() == c.id)
            .ok_or("unknown component")?
            .clone();
        if base.components.contains_key(&id) {
            return Err("component ID collision".into());
        }
        let fields = &manifest
            .type_table
            .get(&id)
            .ok_or("missing property schema")?
            .property_definitions;
        if fields.len() != c.fields.len()
            || fields.iter().zip(&c.fields).any(|(a, b)| a.name != b.name)
        {
            return Err("property schema mismatch".into());
        }
        if c.handlers.iter().any(|h| h.index >= HANDLERS.len()) {
            return Err("handler capacity".into());
        }
        let handlers = c
            .handlers
            .iter()
            .map(|h| {
                rt::HandlerDescriptor::new(
                    Box::leak(h.name.clone().into_boxed_str()),
                    HANDLERS[h.index],
                )
            })
            .collect::<Vec<_>>();
        let d = Box::leak(Box::new(rt::ErasedComponentDescriptor::new(
            Box::leak(c.id.clone().into_boxed_str()),
            &(),
            CREATORS[i],
            apply,
            scope,
            Box::leak(handlers.into_boxed_slice()),
            instantiate,
        )));
        descriptors.insert(id, d);
    }
    manifest.components.extend(base.components);
    manifest.type_table.extend(base.type_table);
    BUNDLE.with(|b| {
        *b.borrow_mut() = Some(Rc::new(Bundle {
            schema,
            traverser: Traverser {
                manifest: RefCell::new(manifest),
                descriptors,
            },
        }))
    });
    Ok(())
}
pub fn capture(ctx: &NodeContext) {
    fn visit(
        node: &Rc<dyn InstanceNode>,
        found: &mut HashMap<TypeId, &'static rt::ErasedComponentDescriptor>,
    ) {
        if let (Some(id), rt::PropertiesScopeInit::Descriptor(d)) = (
            &node.base().template_node_type_id,
            &node.base().properties_scope,
        ) {
            found.insert(id.clone(), *d);
        }
        if let Some(t) = node.get_template() {
            for c in t.borrow().iter() {
                visit(c, found)
            }
        }
        for c in node.base().get_instance_children().borrow().iter() {
            visit(c, found)
        }
    }
    let node = ctx.expanded_node.upgrade().unwrap();
    BASE.with(|b| visit(&node.instance_node.borrow(), &mut b.borrow_mut()));
}
#[pax]
#[primitive("crate::bundle::OutletInstance")]
pub struct DeferredPaxflix {
    pub embedded: Property<bool>,
}
pub struct OutletInstance {
    base: rt::BaseInstance,
}
impl InstanceNode for OutletInstance {
    fn instantiate(args: rt::InstantiationArgs) -> Rc<Self> {
        Rc::new(Self {
            base: rt::BaseInstance::new(
                args,
                rt::InstanceFlags {
                    invisible_to_slot: false,
                    invisible_to_raycasting: true,
                    layer: rt::api::Layer::DontCare,
                    is_component: false,
                    is_slot: false,
                },
            ),
        })
    }
    fn base(&self) -> &rt::BaseInstance {
        &self.base
    }
    fn resolve_debug(
        &self,
        f: &mut std::fmt::Formatter,
        _: Option<&rt::ExpandedNode>,
    ) -> std::fmt::Result {
        f.debug_struct("DeferredPaxflix").finish()
    }
    fn handle_mount(self: Rc<Self>, node: &Rc<rt::ExpandedNode>, ctx: &Rc<rt::RuntimeContext>) {
        let b = bundle();
        let root = b.traverser.get_manifest().main_component_type_id.clone();
        let index = b
            .schema
            .components
            .iter()
            .position(|c| c.id == root.get_unique_identifier())
            .unwrap();
        let mut args = b.traverser.build_component_args(&root);
        let embedded = node
            .properties_scope
            .borrow()
            .get("embedded")
            .unwrap()
            .get_untyped_property()
            .clone();
        args.prototypical_properties = rt::PropertiesInit::Factory(Box::new(move |_, expanded| {
            if expanded.is_some() {
                return None;
            }
            let value = create(index);
            let props = properties(&value);
            let b = bundle();
            let field = b.schema.components[index]
                .fields
                .iter()
                .position(|f| f.name == "embedded")
                .unwrap();
            *bridge::get(props.fields[field]).borrow_mut() =
                fields::Field::Bool(LocalProperty::new_from_untyped(embedded.clone()));
            Some(Rc::new(RefCell::new(value)))
        }));
        args.children = Some(self.base.get_instance_children().clone());
        let instance = instantiate(args);
        let children = node.generate_children(
            vec![(instance, node.stack.clone())],
            ctx,
            &node.parent_frame,
            true,
        );
        node.children.set(children);
    }
}
