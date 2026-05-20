#[allow(unused)]
use crate::*;
use pax_engine::api::{Duration, Property};
use pax_engine::pax;
use pax_runtime::api::NodeContext;

/// Declarative route reader that selects one active [`Route`] child subtree.
///
/// `Router` is a control-flow primitive: it does not render by itself.
/// Instead, it matches the current location against its child `Route` branches
/// and mounts only the winning subtree.
///
/// The active subtree receives an implicit `route` binding with:
///
/// - `route.location`: the location scoped to this router
/// - `route.global_location`: the full browser/native location
/// - `route.params`: named captures from `:param` segments
/// - `route.remainder`: the unmatched tail after this branch
/// - `route.is_exact` and `route.consumed_segments`: match metadata
///
/// Nested routers match against the nearest ancestor `route.remainder` by
/// default, which keeps route trees composable without manual string slicing.
#[pax]
#[engine_import_path("pax_engine")]
#[inlined(<Group/>)]
pub struct Router {}

/// Default route branch shell consumed by a parent [`Router`].
///
/// Use `path` for explicit path matching, `:param` for single-segment capture,
/// and a terminal `*` to consume the remaining tail. Use `default=true` to
/// provide the fallback branch when no explicit path matches.
#[pax]
#[engine_import_path("pax_engine")]
#[route_branch(path = "path", default = "default")]
#[inlined(
    <Group width=100% height=100% anchor_x=0% anchor_y=0%>
        for i in 0..self._projected_children_count {
            slot(i)
        }
    </Group>

    @settings {
        @mount: on_mount
    }
)]
pub struct Route {
    /// Static route pattern, e.g. `/docs/:slug` or `/settings/*`.
    ///
    /// Patterns are matched against the current router scope, not always the
    /// full global path. At the root router, that scope is the full location.
    pub path: Property<String>,
    /// Fallback branch used when no `path` branch matches.
    pub r#default: Property<bool>,
    // Number of slotted children rendered by the route shell.
    pub _projected_children_count: Property<usize>,
}

/// Declarative route branch presented as a card over the current route.
///
/// `RouteCard` is consumed by a parent [`Router`] like [`Route`], but the
/// presentation behavior is owned by this component shell. The route's own
/// contents may still declare additional element or component lifecycle
/// transitions.
#[pax]
#[engine_import_path("pax_engine")]
#[route_branch(path = "path", default = "default")]
#[inlined(
    <Group
        id=card_shell
        width=100%
        height=100%
        anchor_x=0%
        anchor_y=0%
        @in=@timeline {
            duration: {self.duration},
            x: {
                0: {self.edge == RouteCardEdge::Leading ? $base - 100% : self.edge == RouteCardEdge::Trailing ? $base + 100% : $base}, OutQuad,
                100%: {$base},
            },
            y: {
                0: {self.edge == RouteCardEdge::Top ? $base - 100% : self.edge == RouteCardEdge::Bottom ? $base + 100% : $base}, OutQuad,
                100%: {$base},
            },
        }
        @out=@timeline {
            duration: {self.duration},
            x: {
                0: {$base}, OutQuad,
                100%: {self.edge == RouteCardEdge::Leading ? $base - 100% : self.edge == RouteCardEdge::Trailing ? $base + 100% : $base},
            },
            y: {
                0: {$base}, OutQuad,
                100%: {self.edge == RouteCardEdge::Top ? $base - 100% : self.edge == RouteCardEdge::Bottom ? $base + 100% : $base},
            },
        }
    >
        for i in 0..self._projected_children_count {
            slot(i)
        }
    </Group>

    @settings {
        @mount: on_mount
    }
)]
pub struct RouteCard {
    /// Static route pattern, e.g. `/details/:id`.
    pub path: Property<String>,
    /// Fallback card branch used when no `path` branch matches.
    pub r#default: Property<bool>,
    /// Edge from which the card enters and toward which it exits.
    pub edge: Property<RouteCardEdge>,
    /// Duration for the generated card enter/exit transition.
    pub duration: Property<Duration>,
    /// Reserved easing tuning for the card enter/exit transition.
    pub curve: Property<RouteCardCurve>,
    // Number of slotted children rendered by the card shell.
    pub _projected_children_count: Property<usize>,
}

/// Edge used by [`RouteCard`] transitions.
#[pax]
#[engine_import_path("pax_engine")]
pub enum RouteCardEdge {
    Leading,
    #[default]
    Trailing,
    Top,
    Bottom,
}

/// Reserved easing curve values for [`RouteCard`] transition tuning.
#[pax]
#[engine_import_path("pax_engine")]
pub enum RouteCardCurve {
    Linear,
    Hold,
    InQuad,
    #[default]
    OutQuad,
    InOutQuad,
    InBack,
    OutBack,
    InOutBack,
}

impl Route {
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        let projected_children_count = ctx.projected_children_count.clone();
        let deps = [projected_children_count.untyped()];
        self._projected_children_count
            .replace_with(Property::computed(
                move || projected_children_count.get(),
                &deps,
            ));
    }
}

impl RouteCard {
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        let projected_children_count = ctx.projected_children_count.clone();
        let deps = [projected_children_count.untyped()];
        self._projected_children_count
            .replace_with(Property::computed(
                move || projected_children_count.get(),
                &deps,
            ));
    }
}
