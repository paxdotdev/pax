#[allow(unused)]
use crate::*;
use pax_engine::api::{Duration, Opacity, Property};
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

/// Stacked route branch consumed by a parent [`Router`].
///
/// `RouteModal` matches like [`Route`], but it keeps the previously mounted
/// branch active underneath while the modal branch is active. The shell fades a
/// black scrim over the retained branch; the route's own contents are
/// responsible for any modal-specific enter/exit transition.
#[pax]
#[engine_import_path("pax_engine")]
#[custom(Default)]
#[route_branch(path = "path", default = "default", modal = true)]
#[inlined(
    <Group width=100% height=100% anchor_x=0% anchor_y=0%>
        <Group width=100% height=100% anchor_x=0% anchor_y=0%>
            for i in 0..self._projected_children_count {
                slot(i)
            }
        </Group>
        <EventBlocker
            width=100%
            height=100%
            background=rgba(0, 0, 0, 255)
            opacity={self.scrim_opacity}
            @in=@timeline {
                duration: {self.duration},
                opacity: {
                    0: 0, Linear,
                    100%: {$base},
                },
            }
            @out=@timeline {
                duration: {self.duration},
                opacity: {
                    0: {$base}, Linear,
                    100%: 0,
                },
            }
        />
    </Group>

    @settings {
        @mount: on_mount
    }
)]
pub struct RouteModal {
    /// Static route pattern, e.g. `/tools`.
    pub path: Property<String>,
    /// Fallback modal branch used when no `path` branch matches.
    pub r#default: Property<bool>,
    /// Duration for the generated modal scrim enter/exit transition.
    pub duration: Property<Duration>,
    /// Maximum opacity for the black scrim over the retained background.
    /// Unitless values are normalized alpha (`0.3` is 30%); percentages such
    /// as `30%` are also supported.
    pub scrim_opacity: Property<Opacity>,
    // Number of slotted children rendered by the modal route shell.
    pub _projected_children_count: Property<usize>,
}

/// Declarative route branch presented as a card over the current route.
///
/// `RouteCard` is consumed by a parent [`Router`] like [`Route`], but the
/// presentation behavior is owned by this component shell. It retains the
/// previously mounted branch underneath so the incoming or outgoing card slides
/// over stable content while a black scrim fades over the retained branch. The
/// route's own contents may still declare additional element or component
/// lifecycle transitions.
#[pax]
#[engine_import_path("pax_engine")]
#[custom(Default)]
#[route_branch(path = "path", default = "default", modal = true)]
#[inlined(
    <Group width=100% height=100% anchor_x=0% anchor_y=0%>
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
        <EventBlocker
            width=100%
            height=100%
            background=rgba(0, 0, 0, 255)
            opacity={self.scrim_opacity}
            @in=@timeline {
                duration: {self.duration},
                opacity: {
                    0: 0, Linear,
                    100%: {$base},
                },
            }
            @out=@timeline {
                duration: {self.duration},
                opacity: {
                    0: {$base}, Linear,
                    100%: 0,
                },
            }
        />
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
    /// Edge used by the generated card enter/exit transition.
    pub edge: Property<RouteCardEdge>,
    /// Duration for the generated card enter/exit transition.
    pub duration: Property<Duration>,
    /// Maximum opacity for the black scrim over the retained background.
    /// Unitless values are normalized alpha (`0.3` is 30%); percentages such
    /// as `30%` are also supported.
    pub scrim_opacity: Property<Opacity>,
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

impl Default for RouteModal {
    fn default() -> Self {
        Self {
            path: Property::default(),
            r#default: Property::new(false),
            duration: Property::new(Duration::Frames(18.into())),
            scrim_opacity: Property::new(Opacity::Alpha(0.3.into())),
            _projected_children_count: Property::new(0),
        }
    }
}

impl RouteModal {
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

impl Default for RouteCard {
    fn default() -> Self {
        Self {
            path: Property::default(),
            r#default: Property::new(false),
            edge: Property::new(RouteCardEdge::Trailing),
            duration: Property::new(Duration::Frames(18.into())),
            scrim_opacity: Property::new(Opacity::Alpha(0.3.into())),
            curve: Property::new(RouteCardCurve::OutQuad),
            _projected_children_count: Property::new(0),
        }
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
