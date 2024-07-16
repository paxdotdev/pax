use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::rc::Rc;
use pax_manifest::{PaxManifest, TemplateNodeDefinition, TemplateNodeId, TypeId, ValueDefinition};
use pax_runtime_api::CommonProperties;
use pax_runtime_api::pax_value::PaxAny;
use pax_runtime_api::properties::UntypedProperty;
use crate::{ComponentInstance, ExpressionContext, ExpressionTable, HandlerRegistry, InstanceNode, InstantiationArgs, RuntimePropertiesStackFrame};
use crate::api::NodeContext;

pub trait PaxCartridge {
    // fn instantiate_expression_table(&self) -> HashMap<usize, Box<dyn Fn(ExpressionContext) -> PaxAny>>;
    // fn instantiate_main_component(&self) -> Rc<ComponentInstance>;
    // fn get_definition_to_instance_traverser(&self) -> Box<dyn DefinitionToInstanceTraverser>;
}
pub trait DefinitionToInstanceTraverser {
    fn new(manifest: PaxManifest) -> Self where Self: Sized;
    fn get_main_component(&mut self) -> Rc<ComponentInstance>;
    fn get_manifest(&self) -> &PaxManifest;
    fn get_template_node_by_id(&self, id: &str) -> Option<Rc<dyn InstanceNode>>;
    fn get_component(&mut self, type_id: &TypeId) -> Rc<dyn InstanceNode>;
    fn get_component_factory(&self, type_id: &TypeId) -> Option<Box<dyn ComponentFactory>>;
    fn build_component_args(&self, type_id: &TypeId) -> InstantiationArgs;
    fn build_control_flow(&self, containing_component_type_id: &TypeId, node_id: &TemplateNodeId) -> Rc<dyn InstanceNode>;
    fn build_children(&self, containing_component_type_id: &TypeId, node_id: &TemplateNodeId) -> Vec<Rc<dyn InstanceNode>>;
    fn build_template_node(&self, containing_component_type_id: &TypeId, node_id: &TemplateNodeId) -> Rc<dyn InstanceNode>;
    fn check_for_id_in_template_node(&self, id: &str, tnd: &TemplateNodeDefinition) -> bool;
    fn recurse_get_template_node_by_id<'a>(&'a self, id: &str, containing_component_type_id: &'a TypeId, node_id: &'a TemplateNodeId) -> Option<(TypeId, TemplateNodeId)>;
}

pub trait ComponentFactory {
    /// Returns the default CommonProperties factory
    fn build_default_common_properties(&self) -> Box<dyn Fn(Rc<RuntimePropertiesStackFrame>, Rc<ExpressionTable>) -> Rc<RefCell<CommonProperties>>>{
        Box::new(|_,_| Rc::new(RefCell::new(CommonProperties::default())))
    }

    /// Returns the default properties factory for this component
    fn build_default_properties(&self) -> Box<dyn Fn(Rc<RuntimePropertiesStackFrame>, Rc<ExpressionTable>) -> Rc<RefCell<PaxAny>>>;

    /// Returns the CommonProperties factory based on the defined properties
    fn build_inline_common_properties(&self, defined_properties: BTreeMap<String,ValueDefinition>) ->Box<dyn Fn(Rc<RuntimePropertiesStackFrame>, Rc<ExpressionTable>) -> Rc<RefCell<CommonProperties>>>;

    /// Returns the properties factory based on the defined properties
    fn build_inline_properties(&self, defined_properties: BTreeMap<String,ValueDefinition>) -> Box<dyn Fn(Rc<RuntimePropertiesStackFrame>, Rc<ExpressionTable>) -> Rc<RefCell<PaxAny>>>;

    /// Returns the requested closure for the handler registry based on the defined handlers for this component
    /// The argument type is extrapolated based on how the handler was used in the initial compiled template
    fn build_handler(&self, fn_name: &str) -> fn(Rc<RefCell<PaxAny>>, &NodeContext, Option::<PaxAny>);

    /// Returns the handler registry based on the defined handlers for this component
    fn build_component_handlers(&self, handlers: Vec<(String, Vec<String>)>) -> Rc<RefCell<HandlerRegistry>>;

    // Takes a hander registry and adds the given inline handlers to it
    fn add_inline_handlers(&self, handlers: Vec<(String, String)>, registry: Rc<RefCell<HandlerRegistry>>) -> Rc<RefCell<HandlerRegistry>>;

    // Calls the instantion function for the component
    fn build_component(&self, args: InstantiationArgs) -> Rc<dyn InstanceNode>;

    // Returns the property scope for the component
    fn get_properties_scope_factory(&self) -> Box<dyn Fn(Rc<RefCell<PaxAny>>) -> HashMap<String, UntypedProperty>> {
        Box::new(|_| HashMap::new())
    }
}