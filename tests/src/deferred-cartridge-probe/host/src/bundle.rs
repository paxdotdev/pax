//! Fixture-local adapter from portable definitions to the existing Pax runtime.
use crate::*;
use pax_engine::api::{pax_value::PaxAny, Variable};
use pax_engine::pax_manifest::{self as manifest, PaxManifest, TypeId};
use pax_engine::pax_runtime::{self as rt, DefinitionToInstanceTraverser, InstanceNode};
use std::{collections::HashMap, rc::Rc};
#[path = "../../schema.rs"]
mod schema;

thread_local! {
    static PRIMITIVES: RefCell<HashMap<TypeId, &'static rt::ErasedComponentDescriptor>> = RefCell::new(HashMap::new());
    static BUNDLE: RefCell<Option<Rc<Bundle>>> = const { RefCell::new(None) };
    static CONTEXTS: RefCell<Vec<Option<Vec<u32>>>> = const { RefCell::new(Vec::new()) };
}
struct Bundle {
    schema: Rc<schema::Schema>,
    traverser: Traverser,
}
struct Traverser {
    manifest: RefCell<PaxManifest>,
    base: HashMap<TypeId, &'static rt::ErasedComponentDescriptor>,
    descriptor: &'static rt::ErasedComponentDescriptor,
}
impl DefinitionToInstanceTraverser for Traverser {
    fn new(_: PaxManifest) -> Self {
        unreachable!("use validated package registration")
    }
    fn get_manifest(&self) -> std::cell::Ref<'_, PaxManifest> {
        self.manifest.borrow()
    }
    fn get_component_descriptor(
        &self,
        id: &TypeId,
    ) -> Option<&'static rt::ErasedComponentDescriptor> {
        if id == &self.manifest.borrow().main_component_type_id {
            Some(self.descriptor)
        } else {
            self.base.get(id).copied()
        }
    }
}

#[derive(Clone)]
enum Field {
    Number(LocalProperty<u32>),
    Boolean(LocalProperty<bool>),
}
struct Properties {
    fields: HashMap<String, Field>,
    schema: Rc<schema::Schema>,
    context: u32,
    token: Cell<u32>,
}
fn properties(value: &PaxAny) -> &Properties {
    match value {
        PaxAny::Any(value) => value.downcast_ref().unwrap(),
        _ => unreachable!(),
    }
}
fn scope(_: &'static (dyn std::any::Any + Sync), value: &PaxAny) -> HashMap<String, Variable> {
    properties(value)
        .fields
        .iter()
        .map(|(name, field)| {
            (
                name.clone(),
                match field {
                    Field::Number(p) => Variable::new_from_typed_property(p.clone()),
                    Field::Boolean(p) => Variable::new_from_typed_property(p.clone()),
                },
            )
        })
        .collect()
}
fn invoke(index: usize, value: Rc<RefCell<PaxAny>>) {
    let value = value.borrow();
    let props = properties(&value);
    let handler = &props.schema.handlers[index];
    match handler.kind.as_str() {
        "mount" => {
            let token = call(handler.opcode, props.context, 0);
            assert_ne!(token, 0, "child instance capacity exceeded");
            props.token.set(token);
            LAST_INSTANCE.with(|t| t.set(token));
            MOUNTS.with(|n| n.set(n.get() + 1));
        }
        "unmount" => {
            call(handler.opcode, props.token.replace(0), 0);
            let handles = CONTEXTS.with(|c| c.borrow_mut()[props.context as usize - 1].take());
            if let Some(handles) = handles {
                VALUES.with(|values| {
                    let mut values = values.borrow_mut();
                    for handle in handles.into_iter().filter(|h| *h != 0) {
                        values[handle as usize - 1] = None;
                    }
                });
            }
            UNMOUNTS.with(|n| n.set(n.get() + 1));
        }
        _ => {
            call(handler.opcode, props.token.get(), 0);
        }
    }
}
// Existing HandlerDescriptor stores fn pointers, not closures. A bounded bank
// supplies generic trampolines; names and opcodes arrive in the bundle.
macro_rules! handlers {
    ($($name:ident = $index:literal),*) => {
        $(fn $name(value: Rc<RefCell<PaxAny>>, _: &NodeContext, _: Option<PaxAny>) { invoke($index, value); })*
        const HANDLERS: &[fn(Rc<RefCell<PaxAny>>, &NodeContext, Option<PaxAny>)] = &[$($name),*];
    }
}
handlers!(
    h0 = 0,
    h1 = 1,
    h2 = 2,
    h3 = 3,
    h4 = 4,
    h5 = 5,
    h6 = 6,
    h7 = 7
);

/// Copy and validate the data portion before allowing any instance to mount.
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn probe_stage(definition: &[u8], descriptor: &str) -> Result<(), wasm_bindgen::JsValue> {
    stage(definition, descriptor).map_err(|e| wasm_bindgen::JsValue::from_str(&e))
}
fn stage(definition: &[u8], descriptor: &str) -> Result<(), String> {
    if BUNDLE.with(|b| b.borrow().is_some()) {
        return Err("Package already staged".into());
    }
    let mut manifest = manifest::binary::from_slice(definition).map_err(|e| e.to_string())?;
    let schema: schema::Schema = serde_json::from_str(descriptor).map_err(|e| e.to_string())?;
    let reject = |message: &str| Err(message.to_string());
    if schema.abi != 2 || schema.handlers.len() > HANDLERS.len() || schema.fields.len() > 32 {
        return reject("Unsupported package ABI or descriptor capacity");
    }
    let root = manifest.main_component_type_id.clone();
    if root.get_unique_identifier() != schema.component || manifest.components.len() != 1 {
        return reject("Expected exactly one packaged component");
    }
    let component = manifest
        .components
        .get(&root)
        .ok_or("Missing component definition")?;
    if component.is_primitive
        || component.is_struct_only_component
        || !component.timelines.is_empty()
    {
        return reject("Probe expects a component without self timelines");
    }
    let fields = &manifest
        .type_table
        .get(&root)
        .ok_or("Missing property schema")?
        .property_definitions;
    if fields.len() != schema.fields.len() {
        return reject("Property schema mismatch");
    }
    let mut names = std::collections::HashSet::new();
    for (field, ty) in schema.fields.iter().zip(fields) {
        if !names.insert(&field.name)
            || field.name != ty.name
            || ty.type_id != TypeId::build_primitive(&field.kind)
            || !matches!(field.kind.as_str(), "u32" | "bool")
        {
            return reject("Unsupported or inconsistent property schema");
        }
        if let Some(import) = &field.import {
            if !matches!(
                (import.as_str(), field.kind.as_str()),
                ("count" | "step", "u32") | ("warm", "bool")
            ) {
                return reject("Component import contract mismatch");
            }
        }
    }
    let mut names = std::collections::HashSet::new();
    for handler in &schema.handlers {
        if !names.insert(&handler.name)
            || handler.opcode == 0
            || !matches!(handler.kind.as_str(), "mount" | "unmount" | "event")
        {
            return reject("Invalid handler descriptor");
        }
    }
    if manifest
        .get_component_handlers(&root)
        .iter()
        .any(|(_, routes)| routes.iter().any(|name| !names.contains(name)))
    {
        return reject("Unknown component handler");
    }
    for kind in ["mount", "unmount"] {
        if schema.handlers.iter().filter(|h| h.kind == kind).count() != 1 {
            return reject("Expected one create and one destroy handler");
        }
        let binding = manifest.get_component_handlers(&root);
        let route = binding
            .iter()
            .find(|(event, _)| event == kind)
            .ok_or("Missing lifecycle route")?;
        if route.1.len() != 1
            || !schema
                .handlers
                .iter()
                .any(|h| h.kind == kind && h.name == route.1[0])
        {
            return reject("Lifecycle route mismatch");
        }
    }
    let base_manifest = crate::init_manifest_for("crate::Probe");
    let base = PRIMITIVES.with(|p| p.borrow().clone());
    if base_manifest.components.contains_key(&root) {
        return reject("Component ID collision");
    }
    for node in component
        .template
        .as_ref()
        .ok_or("Missing template")?
        .get_nodes()
    {
        match node.type_id.get_pax_type() {
            manifest::PaxType::If
            | manifest::PaxType::Slot
            | manifest::PaxType::Repeat
            | manifest::PaxType::Comment => (),
            _ if base.contains_key(&node.type_id) => (),
            _ => return reject("Unavailable component dependency"),
        }
        if manifest
            .get_inline_event_handlers(node)
            .iter()
            .any(|(_, name)| !names.contains(name))
        {
            return reject("Unknown event handler");
        }
    }
    manifest.components.extend(base_manifest.components);
    manifest.type_table.extend(base_manifest.type_table);
    // The existing descriptor ABI requires static references. This fixture accepts
    // one package for the page lifetime, so these bounded allocations live with it.
    let descriptors = schema
        .handlers
        .iter()
        .enumerate()
        .map(|(i, h)| rt::HandlerDescriptor {
            name: Box::leak(h.name.clone().into_boxed_str()),
            function: HANDLERS[i],
        })
        .collect::<Vec<_>>()
        .into_boxed_slice();
    let descriptor = Box::leak(Box::new(rt::ErasedComponentDescriptor::new(
        Box::leak(schema.component.clone().into_boxed_str()),
        &(),
        || unreachable!("package properties use the instance factory"),
        |_, _, _, _, _| {},
        scope,
        Box::leak(descriptors),
        |args| rt::ComponentInstance::instantiate(args),
    )));
    BUNDLE.with(|b| {
        *b.borrow_mut() = Some(Rc::new(Bundle {
            schema: Rc::new(schema),
            traverser: Traverser {
                manifest: RefCell::new(manifest),
                base,
                descriptor,
            },
        }))
    });
    Ok(())
}
// Bootstrap from public runtime descriptors already used by this host. A real
// package loader should receive an explicit primitive/capability registry from
// codegen; the probe only admits primitives already present in the base tree.
pub fn capture_primitives(ctx: &NodeContext) {
    fn visit(
        node: &Rc<dyn InstanceNode>,
        manifest: &PaxManifest,
        found: &mut HashMap<TypeId, &'static rt::ErasedComponentDescriptor>,
    ) {
        if let (Some(id), rt::PropertiesScopeInit::Descriptor(descriptor)) = (
            &node.base().template_node_type_id,
            &node.base().properties_scope,
        ) {
            if manifest.components.get(id).is_some_and(|c| c.is_primitive) {
                found.insert(id.clone(), *descriptor);
            }
        }
        if let Some(template) = node.get_template() {
            for child in template.borrow().iter() {
                visit(child, manifest, found);
            }
        }
        for child in node.base().get_instance_children().borrow().iter() {
            visit(child, manifest, found);
        }
    }
    let node = ctx.expanded_node.upgrade().unwrap();
    let manifest = crate::init_manifest_for("crate::Probe");
    PRIMITIVES.with(|p| visit(&node.instance_node.borrow(), &manifest, &mut p.borrow_mut()));
}

pub fn staged() -> bool {
    BUNDLE.with(|b| b.borrow().is_some())
}
pub fn stage_embedded() {
    #[cfg(feature = "monolithic")]
    stage(
        include_bytes!(env!("PROBE_DEFINITION")),
        include_str!(env!("PROBE_DESCRIPTOR")),
    )
    .unwrap();
}
#[no_mangle]
pub extern "C" fn probe_bind(context: u32, field: u32) -> u32 {
    CONTEXTS.with(|c| {
        c.borrow()
            .get(context.wrapping_sub(1) as usize)
            .and_then(|handles| handles.as_ref())
            .and_then(|h| h.get(field as usize))
            .copied()
            .unwrap_or(0)
    })
}

#[pax]
#[primitive("crate::bundle::OutletInstance")]
pub struct DeferredOutlet {
    pub count: Property<u32>,
    pub step: Property<u32>,
    pub warm: Property<bool>,
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
        f.debug_struct("DeferredOutlet").finish()
    }
    fn handle_mount(self: Rc<Self>, node: &Rc<rt::ExpandedNode>, ctx: &Rc<rt::RuntimeContext>) {
        let bundle = BUNDLE.with(|b| b.borrow().as_ref().unwrap().clone());
        let imports = node.properties_scope.borrow();
        let mut fields = HashMap::new();
        let mut handles = Vec::new();
        for field in &bundle.schema.fields {
            let imported = field
                .import
                .as_ref()
                .map(|name| imports.get(name).unwrap().get_untyped_property().clone());
            let value = match field.kind.as_str() {
                "u32" => Field::Number(
                    imported
                        .map(LocalProperty::new_from_untyped)
                        .unwrap_or_else(|| LocalProperty::new(field.default)),
                ),
                "bool" => Field::Boolean(
                    imported
                        .map(LocalProperty::new_from_untyped)
                        .unwrap_or_else(|| LocalProperty::new(field.default != 0)),
                ),
                _ => unreachable!(),
            };
            handles.push(match &value {
                Field::Number(p) => VALUES.with(|values| {
                    let mut v = values.borrow_mut();
                    v.push(Some(p.clone()));
                    v.len() as u32
                }),
                _ => 0,
            });
            fields.insert(field.name.clone(), value);
        }
        let context = CONTEXTS.with(|c| {
            let mut c = c.borrow_mut();
            c.push(Some(handles));
            c.len() as u32
        });
        let properties = Rc::new(RefCell::new(PaxAny::Any(Box::new(Properties {
            fields,
            schema: bundle.schema.clone(),
            context,
            token: Cell::new(0),
        }))));
        let root = bundle
            .traverser
            .get_manifest()
            .main_component_type_id
            .clone();
        let mut args = bundle.traverser.build_component_args(&root);
        args.prototypical_properties = rt::PropertiesInit::Factory(Box::new(move |_, node| {
            node.is_none().then(|| properties.clone())
        }));
        args.children = Some(self.base.get_instance_children().clone());
        let instance: Rc<dyn InstanceNode> = rt::ComponentInstance::instantiate(args);
        let children = node.generate_children(
            vec![(instance, node.stack.clone())],
            ctx,
            &node.parent_frame,
            true,
        );
        node.children.set(children);
    }
}
