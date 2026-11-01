use std::{
    any::{Any, TypeId},
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::{Rc, Weak},
};

use crate::{api::NodeContext, ExpandedNode, RuntimeContext, RuntimePropertiesStackFrame};
use pax_runtime_api::{
    properties::UntypedProperty, Interpolatable, LocalProperty, PaxValue, Store, ToPaxValue,
    Variable,
};

const PRESENTATION_SYMBOL: &str = "@route_presentation";

#[derive(Default)]
pub(super) struct PresentationScope {
    pub revision: LocalProperty<u64>,
    owner: RefCell<Weak<ExpandedNode>>,
    context: RefCell<Weak<RuntimeContext>>,
    stores: RefCell<HashMap<TypeId, Rc<dyn Any>>>,
}

impl PresentationScope {
    pub fn bind(&self, owner: &Rc<ExpandedNode>, context: &Rc<RuntimeContext>) {
        *self.owner.borrow_mut() = Rc::downgrade(owner);
        *self.context.borrow_mut() = Rc::downgrade(context);
    }

    pub fn clear(&self) {
        self.stores.borrow_mut().clear();
    }

    pub fn branch_stack(
        self: &Rc<Self>,
        stack: &Rc<RuntimePropertiesStackFrame>,
    ) -> Rc<RuntimePropertiesStackFrame> {
        let branch = RoutePresentation(Rc::new(BranchPresentation {
            scope: self.clone(),
            ..Default::default()
        }));
        stack.push(HashMap::from([(
            PRESENTATION_SYMBOL.into(),
            Variable::new_from_typed_property(LocalProperty::new(branch)),
        )]))
    }
}

#[derive(Default)]
struct BranchPresentation {
    scope: Rc<PresentationScope>,
    selected: Cell<bool>,
    retained: Cell<bool>,
    on_select: RefCell<Option<Rc<dyn Fn()>>>,
}

/// Mounted route participation in a presentation shared by one Router.
///
/// Custom route shells can retain live siblings while coordinating motion. The
/// selected route still follows the location immediately. Retention keeps the
/// subtree mounted and interactive; the shell must clip content that is hidden.
/// Release retention when the presentation no longer needs a branch. Ordinary
/// exit timelines then run before final unmount. This is not a persistent cache.
#[derive(Clone, Default)]
pub struct RoutePresentation(Rc<BranchPresentation>);

impl Interpolatable for RoutePresentation {}
impl ToPaxValue for RoutePresentation {
    fn to_pax_value(self) -> PaxValue {
        PaxValue::Object(vec![])
    }
}
impl PartialEq for RoutePresentation {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

/// Non-owning reference to a participating route, expiring with its instance.
#[derive(Clone)]
pub struct WeakRoutePresentation(Weak<BranchPresentation>);

impl WeakRoutePresentation {
    /// Upgrade while the corresponding route instance remains mounted or retained.
    pub fn upgrade(&self) -> Option<RoutePresentation> {
        self.0.upgrade().map(RoutePresentation)
    }
}

impl RoutePresentation {
    pub(super) fn from_stack(stack: &RuntimePropertiesStackFrame) -> Option<Self> {
        stack
            .resolve_symbol_as_erased_property(PRESENTATION_SYMBOL)
            .map(|property| LocalProperty::<Self>::new_from_untyped(property).get())
    }

    pub(super) fn select(&self, selected: bool) {
        if self.0.selected.replace(selected) != selected && selected {
            let callback = self.0.on_select.borrow().clone();
            if let Some(callback) = callback {
                callback();
            }
        }
    }

    pub(super) fn retained(&self) -> bool {
        self.0.retained.get()
    }
    pub(super) fn participates(&self) -> bool {
        self.0.on_select.borrow().is_some()
    }

    /// Whether this instance currently matches its Router's location.
    pub fn is_selected(&self) -> bool {
        self.0.selected.get()
    }

    /// Whether this shell still requests presentation retention.
    pub fn retained_for_presentation(&self) -> bool {
        self.retained()
    }

    /// Keep this instance mounted after another branch is selected, or release it.
    pub fn retain(&self, retained: bool) {
        if self.0.retained.replace(retained) != retained {
            let revision = &self.0.scope.revision;
            revision.set(revision.get().wrapping_add(1));
        }
    }

    /// Register the shell's selection callback. It runs immediately if selected,
    /// and again when the same retained instance is selected during motion.
    /// Capture a weak handle in the callback to avoid retaining this instance.
    pub fn on_select(&self, callback: impl Fn() + 'static) {
        let callback: Rc<dyn Fn()> = Rc::new(callback);
        *self.0.on_select.borrow_mut() = Some(callback.clone());
        if self.is_selected() {
            callback();
        }
    }

    /// Capture a non-owning handle in controller state and callbacks.
    pub fn downgrade(&self) -> WeakRoutePresentation {
        WeakRoutePresentation(Rc::downgrade(&self.0))
    }

    /// One controller per concrete store type and Router instance. Nested and
    /// sibling Routers receive independent stores; final Router unmount drops them.
    pub fn shared<T: Store + Default>(&self) -> Rc<RefCell<T>> {
        let owner = self
            .0
            .scope
            .owner
            .borrow()
            .upgrade()
            .expect("route presentation requires a live Router");
        self.0
            .scope
            .stores
            .borrow_mut()
            .entry(TypeId::of::<T>())
            // The first registering shell can retire before later turns. Shared
            // properties belong to the Router's binding scope, not that shell.
            .or_insert_with(|| owner.with_binding_scope(|| Rc::new(RefCell::new(T::default()))))
            .clone()
            .downcast::<RefCell<T>>()
            .expect("router store type key")
    }

    /// Subscribe for the Router's mounted lifetime, independent of face turnover.
    /// Use animation properties as dependencies so settled presentations do not
    /// remain subscribed to a frame clock. Avoid strong cycles through shared stores.
    pub fn subscribe(&self, dependencies: &[UntypedProperty], effect: impl Fn() + 'static) {
        if let (Some(owner), Some(context)) = (
            self.0.scope.owner.borrow().upgrade(),
            self.0.scope.context.borrow().upgrade(),
        ) {
            let property = owner.with_binding_scope(|| {
                context.register_node_effect(owner.id, dependencies, effect)
            });
            owner.subscriptions.borrow_mut().push(property);
        }
    }
}

impl NodeContext {
    /// Presentation handle for the nearest enclosing Router branch, when any.
    /// Register custom route presentation during the shell's mount handler.
    pub fn route_presentation(&self) -> Option<RoutePresentation> {
        RoutePresentation::from_stack(&self.expression_stack)
    }
}
