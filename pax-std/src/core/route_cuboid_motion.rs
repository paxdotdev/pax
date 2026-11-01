//! Shared rigid-face motion; layout projects this normalized cube into the viewport.

use super::router::{RouteCuboidDirection, RouteCuboidPose};
use pax_engine::api::{Duration, EasingCurve, LocalProperty, Store};
use pax_runtime::router::{RoutePresentation, WeakRoutePresentation};
use std::rc::Rc;

#[derive(Clone, Copy, Debug, PartialEq)]
struct Orientation([f64; 4]);

impl Default for Orientation {
    fn default() -> Self {
        Self([1., 0., 0., 0.])
    }
}

#[cfg(test)]
mod tests {
    use super::super::router::RouteCuboid;
    use super::*;
    use pax_engine::api::{
        pax_value::{PaxAny, ToFromPaxAny},
        properties, CommonProperties,
    };
    use pax_manifest::ControlFlowRouteBranchDefinition;
    use pax_runtime::{
        compile_route_branches, BaseInstance, CommonPropertiesInit, ComponentInstance,
        ExpandedNode, InstanceFlags, InstanceNode, InstantiationArgs, PropertiesInit,
        PropertiesScopeInit, RouteLocation, RouterInstance, RouterProperties, RuntimeContext,
    };
    use std::cell::RefCell;

    fn near(a: f64, b: f64) {
        assert!((a - b).abs() < 1e-6, "{a} != {b}");
    }

    #[test]
    fn retarget_preserves_pose_and_angular_velocity() {
        let first = Motion {
            to: quarter(RouteCuboidDirection::Left),
            duration_ms: 1200.,
            ..Default::default()
        };
        for progress in [0.05, 0.25, 0.5, 0.9] {
            let from = first.sample(progress);
            let velocity = first.velocity(progress);
            for destination in [
                Orientation::default(),
                quarter(RouteCuboidDirection::Up),
                quarter(RouteCuboidDirection::Left).multiply(first.to),
            ] {
                let next = Motion {
                    from,
                    to: destination,
                    tangent: velocity.map(|v| v * 600.),
                    duration_ms: 600.,
                };
                near(norm(next.sample(0.).multiply(from.inverse()).log()), 0.);
                near(
                    norm(next.sample(1.).multiply(destination.inverse()).log()),
                    0.,
                );
                for (a, b) in next.velocity(0.).into_iter().zip(velocity) {
                    assert!((a - b).abs() < 1e-6);
                }
                near(norm(next.velocity(1.)), 0.);
            }
        }
    }

    #[test]
    fn euler_presentation_matches_quaternion_across_axis_changes_and_poles() {
        let mut q = Orientation::default();
        for i in 0..160 {
            q = quarter(match i % 4 {
                0 => RouteCuboidDirection::Left,
                1 => RouteCuboidDirection::Up,
                2 => RouteCuboidDirection::Right,
                _ => RouteCuboidDirection::Down,
            })
            .multiply(q);
            for pose in [q, Orientation::exp([0.13, 0.29, -0.07]).multiply(q)] {
                let [rx, ry, rz] = pose.euler().map(f64::to_radians);
                let rebuilt = Orientation::exp([0., 0., rz])
                    .multiply(Orientation::exp([0., ry, 0.]))
                    .multiply(Orientation::exp([rx, 0., 0.]));
                near(norm(rebuilt.multiply(pose.inverse()).log()), 0.);
            }
        }
    }

    struct Shell {
        base: BaseInstance,
    }
    impl InstanceNode for Shell {
        fn instantiate(args: InstantiationArgs) -> Rc<Self> {
            Rc::new(Self {
                base: BaseInstance::new(
                    args,
                    InstanceFlags {
                        invisible_to_slot: false,
                        invisible_to_raycasting: true,
                        layer: pax_runtime::api::Layer::DontCare,
                        is_component: true,
                        is_slot: false,
                    },
                ),
            })
        }
        fn base(&self) -> &BaseInstance {
            &self.base
        }
        fn resolve_debug(
            &self,
            f: &mut std::fmt::Formatter,
            _: Option<&ExpandedNode>,
        ) -> std::fmt::Result {
            write!(f, "Cuboid test shell")
        }
        fn handle_mount(self: Rc<Self>, node: &Rc<ExpandedNode>, ctx: &Rc<RuntimeContext>) {
            node.with_properties_unwrapped(|route: &mut RouteCuboid| {
                route.on_mount(&node.get_node_context(ctx))
            });
        }
    }

    fn args() -> InstantiationArgs {
        InstantiationArgs {
            prototypical_common_properties: CommonPropertiesInit::Factory(Box::new(|_, node| {
                node.is_none()
                    .then(|| Rc::new(RefCell::new(CommonProperties::default())))
            })),
            prototypical_properties: PropertiesInit::Factory(Box::new(|_, node| {
                node.is_none()
                    .then(|| Rc::new(RefCell::new(PaxAny::Builtin(Default::default()))))
            })),
            properties_scope: PropertiesScopeInit::None,
            handler_registry: None,
            children: None,
            component_template: None,
            component_settings: None,
            component_camera: None,
            template_node_identifier: None,
            template_node_type_id: None,
            template_node_selector_info: None,
            transition_config: Default::default(),
        }
    }

    struct Harness {
        _root: Rc<ExpandedNode>,
        router: Rc<ExpandedNode>,
        context: Rc<RuntimeContext>,
        location: LocalProperty<RouteLocation>,
        direction: LocalProperty<RouteCuboidDirection>,
        duration: LocalProperty<Duration>,
    }
    impl Harness {
        fn new() -> Self {
            let context = Rc::new(crate::test_support::runtime_context());
            properties::register_time(&context.globals().elapsed_frames);
            properties::register_millis(&context.globals().elapsed_millis);
            let location = LocalProperty::new(RouteLocation::root());
            let direction = LocalProperty::new(RouteCuboidDirection::Left);
            let duration = LocalProperty::new(Duration::Milliseconds(1200.into()));
            let mut shell_args = args();
            let d = direction.clone();
            let t = duration.clone();
            shell_args.prototypical_properties =
                PropertiesInit::Factory(Box::new(move |_, node| {
                    node.is_none().then(|| {
                        let shell = RouteCuboid::default();
                        shell.direction.replace_with(d.clone());
                        shell.duration.replace_with(t.clone());
                        Rc::new(RefCell::new(shell.to_pax_any()))
                    })
                }));
            let shell = Shell::instantiate(shell_args);
            let mut router_args = args();
            let input = location.clone();
            router_args.prototypical_properties =
                PropertiesInit::Factory(Box::new(move |_, node| {
                    node.is_none().then(|| {
                        let mut props = RouterProperties::default();
                        props.input_location = input.clone();
                        props.global_location = input.clone();
                        Rc::new(RefCell::new(props.to_pax_any()))
                    })
                }));
            router_args.children = Some(RefCell::new(vec![shell.clone(), shell]));
            let router = RouterInstance::instantiate_with_branches(
                router_args,
                compile_route_branches(&[
                    ControlFlowRouteBranchDefinition {
                        path: Some("/".into()),
                        ..Default::default()
                    },
                    ControlFlowRouteBranchDefinition {
                        path: Some("/:page".into()),
                        ..Default::default()
                    },
                ]),
                vec![0..1, 1..2],
            );
            let mut root_args = args();
            root_args.component_template = Some(RefCell::new(vec![router]));
            let root =
                ExpandedNode::initialize_root(ComponentInstance::instantiate(root_args), &context);
            root.recurse_update(&context);
            context.drain_node_effects();
            let router = root.children.get()[0].clone();
            Self {
                _root: root,
                router,
                context,
                location,
                direction,
                duration,
            }
        }
        fn go(&self, path: &str) {
            self.location.set(RouteLocation {
                path_segments: if path.is_empty() {
                    vec![]
                } else {
                    vec![path.into()]
                },
                ..Default::default()
            });
            self.router.recurse_update(&self.context);
            self.context.drain_node_effects();
        }
        fn time(&self, millis: u64) {
            self.context.globals().elapsed_millis.set(millis);
            self.context.globals().elapsed_frames.set(millis / 16);
            self.context.drain_node_effects();
            self.router.recurse_update(&self.context);
            self.context.drain_node_effects();
        }
        fn nodes(&self) -> Vec<Rc<ExpandedNode>> {
            self.router.mounted_children.borrow().clone()
        }
        fn pose(node: &Rc<ExpandedNode>) -> RouteCuboidPose {
            node.with_properties_unwrapped(|shell: &mut RouteCuboid| shell._pose.get())
        }
    }

    #[test]
    fn three_routes_share_rigid_faces_and_reversal_preserves_instance_and_pose() {
        let h = Harness::new();
        let a = h.nodes()[0].clone();
        h.go("b");
        h.time(300);
        assert_eq!(h.nodes().len(), 2);
        let ap = Harness::pose(&a);
        h.go("c");
        assert_eq!(h.nodes().len(), 3);
        let after = Harness::pose(&a);
        near(ap.x, after.x);
        near(ap.z, after.z);
        h.time(500);
        let ap = Harness::pose(&a);
        h.go("");
        assert_eq!(h.router.active_children.borrow()[0].id, a.id);
        let after = Harness::pose(&a);
        near(ap.x, after.x);
        near(ap.z, after.z);
        let normals = h
            .nodes()
            .iter()
            .map(|node| {
                let p = Harness::pose(node);
                [p.x, p.y, p.z]
            })
            .collect::<Vec<_>>();
        for (i, a) in normals.iter().enumerate() {
            for b in &normals[i + 1..] {
                let dot = (0..3).map(|j| a[j] * b[j]).sum::<f64>();
                assert!(
                    dot.abs() < 1e-6 || (dot + 1.).abs() < 1e-6,
                    "faces separated: {dot}"
                );
            }
        }
        h.time(1700);
        assert_eq!(h.nodes().len(), 1);
        assert_eq!(h.nodes()[0].id, a.id);
        near(Harness::pose(&a).z, 1.);
    }

    #[test]
    fn setting_edits_are_sampled_on_navigation_and_zero_duration_releases_faces() {
        let h = Harness::new();
        h.go("b");
        h.time(300);
        h.direction.set(RouteCuboidDirection::Up);
        h.duration.set(Duration::Milliseconds(0.into()));
        h.time(600);
        assert_eq!(h.nodes().len(), 2);
        let p = Harness::pose(&h.nodes()[0]);
        near(p.y, 0.);
        assert!(p.x.abs() > 0.1);
        h.go("c");
        assert_eq!(h.nodes().len(), 1);
        near(Harness::pose(&h.nodes()[0]).z, 1.);
    }

    #[test]
    fn controller_outlives_its_first_face_and_repeated_settled_turns() {
        let h = Harness::new();
        for i in 1..=8 {
            h.go(&format!("page-{i}"));
            h.time(i * 1200 - 600);
            assert_eq!(h.nodes().len(), 2);
            assert!(Harness::pose(&h.nodes()[0]).x.abs() > 0.1);
            h.time(i * 1200);
            assert_eq!(h.nodes().len(), 1);
            near(Harness::pose(&h.nodes()[0]).z, 1.);
        }
    }

    #[test]
    fn settled_controller_disconnects_clocks_and_router_unmount_releases_it() {
        let h = Harness::new();
        let handle = h.nodes()[0]
            .get_node_context(&h.context)
            .route_presentation()
            .unwrap();
        let controller = handle.shared::<CuboidMotion>();
        let weak = Rc::downgrade(&controller);
        h.go("b");
        h.time(1220);
        h.time(1240);
        for clock in [
            h.context.globals().elapsed_frames,
            h.context.globals().elapsed_millis,
        ] {
            assert!(!properties::property_has_direct_outbound(
                &clock.untyped(),
                &controller.borrow().progress.untyped()
            ));
        }
        drop(controller);
        h._root.clone().recurse_unmount(&h.context);
        assert!(weak.upgrade().is_none());
    }

    #[test]
    fn repeated_arbitrary_routes_keep_a_bounded_pool_and_release_nodes() {
        let h = Harness::new();
        let mut weak_nodes = Vec::new();
        for i in 1..=120 {
            h.time(i * 35);
            h.direction.set(match i % 4 {
                0 => RouteCuboidDirection::Left,
                1 => RouteCuboidDirection::Up,
                2 => RouteCuboidDirection::Right,
                _ => RouteCuboidDirection::Down,
            });
            h.go(&format!("route-{i}"));
            assert!(h.nodes().len() <= 6, "{} live faces", h.nodes().len());
            weak_nodes.push(Rc::downgrade(&h.nodes()[0]));
        }
        h.time(5400);
        assert_eq!(h.nodes().len(), 1);
        assert_eq!(
            weak_nodes
                .iter()
                .filter(|node| node.upgrade().is_some())
                .count(),
            1
        );
    }
}

impl Orientation {
    fn multiply(self, rhs: Self) -> Self {
        let [a, b, c, d] = self.0;
        let [w, x, y, z] = rhs.0;
        Self([
            a * w - b * x - c * y - d * z,
            a * x + b * w + c * z - d * y,
            a * y - b * z + c * w + d * x,
            a * z + b * y - c * x + d * w,
        ])
        .normalized()
    }
    fn normalized(self) -> Self {
        let length = self.0.iter().map(|x| x * x).sum::<f64>().sqrt();
        Self(self.0.map(|x| x / length))
    }
    fn inverse(self) -> Self {
        let [w, x, y, z] = self.0;
        Self([w, -x, -y, -z])
    }
    fn exp(vector: [f64; 3]) -> Self {
        let angle = norm(vector);
        if angle < 1e-12 {
            return Self::default();
        }
        let k = (angle / 2.).sin() / angle;
        Self([
            (angle / 2.).cos(),
            vector[0] * k,
            vector[1] * k,
            vector[2] * k,
        ])
    }
    fn log(self) -> [f64; 3] {
        let [mut w, mut x, mut y, mut z] = self.0;
        if w < -1e-10 {
            w = -w;
            x = -x;
            y = -y;
            z = -z;
        }
        let length = norm([x, y, z]);
        if length < 1e-12 {
            return [0.; 3];
        }
        let k = 2. * length.atan2(w) / length;
        [x * k, y * k, z * k]
    }
    fn normal(self) -> [f64; 3] {
        let [w, x, y, z] = self.0;
        [
            2. * (x * z + w * y),
            2. * (y * z - w * x),
            1. - 2. * (x * x + y * y),
        ]
    }
    fn euler(self) -> [f64; 3] {
        // Rz * Ry * Rx, matching Pax's spatial layout transform. Decomposition
        // is only for presentation; motion and face identity stay quaternion-based.
        let [w, x, y, z] = self.0;
        let sy = (2. * (w * y - z * x)).clamp(-1., 1.);
        let ry = sy.asin();
        let (rx, rz) = if sy.abs() < 1. - 1e-10 {
            (
                (2. * (w * x + y * z)).atan2(1. - 2. * (x * x + y * y)),
                (2. * (w * z + x * y)).atan2(1. - 2. * (y * y + z * z)),
            )
        } else {
            (0., (-2. * (x * y - w * z)).atan2(1. - 2. * (x * x + z * z)))
        };
        [rx.to_degrees(), ry.to_degrees(), rz.to_degrees()]
    }
}

fn norm(v: [f64; 3]) -> f64 {
    v.iter().map(|x| x * x).sum::<f64>().sqrt()
}

fn quarter(direction: RouteCuboidDirection) -> Orientation {
    let a = std::f64::consts::FRAC_PI_2;
    Orientation::exp(match direction {
        RouteCuboidDirection::Left => [0., -a, 0.],
        RouteCuboidDirection::Right => [0., a, 0.],
        RouteCuboidDirection::Up => [a, 0., 0.],
        RouteCuboidDirection::Down => [-a, 0., 0.],
    })
}

#[derive(Default)]
struct Motion {
    from: Orientation,
    to: Orientation,
    tangent: [f64; 3],
    duration_ms: f64,
}

impl Motion {
    fn sample(&self, t: f64) -> Orientation {
        let t = t.clamp(0., 1.);
        let end = self.to.multiply(self.from.inverse()).log();
        let h10 = t * (1. - t) * (1. - t);
        let h01 = t * t * (3. - 2. * t);
        Orientation::exp(std::array::from_fn(|i| {
            h10 * self.tangent[i] + h01 * end[i]
        }))
        .multiply(self.from)
    }
    fn velocity(&self, t: f64) -> [f64; 3] {
        if self.duration_ms <= 0. || t >= 1. {
            return [0.; 3];
        }
        let lo = (t - 1e-5).max(0.);
        let hi = (t + 1e-5).min(1.);
        self.sample(hi)
            .multiply(self.sample(lo).inverse())
            .log()
            .map(|x| x / ((hi - lo) * self.duration_ms))
    }
}

struct Face {
    route: WeakRoutePresentation,
    orientation: Orientation,
    pose: LocalProperty<RouteCuboidPose>,
}

#[derive(Default)]
pub(super) struct CuboidMotion {
    progress: LocalProperty<f64>,
    subscribed: bool,
    motion: Motion,
    faces: Vec<Face>,
}

impl Store for CuboidMotion {}

impl CuboidMotion {
    pub fn register(
        route: &RoutePresentation,
        pose: LocalProperty<RouteCuboidPose>,
        direction: LocalProperty<RouteCuboidDirection>,
        duration: LocalProperty<Duration>,
    ) {
        let controller = route.shared::<Self>();
        if !controller.borrow().subscribed {
            controller.borrow_mut().subscribed = true;
            let progress = controller.borrow().progress.clone();
            let weak = Rc::downgrade(&controller);
            route.subscribe(&[progress.untyped()], move || {
                if let Some(controller) = weak.upgrade() {
                    controller.borrow_mut().present();
                }
            });
        }
        let weak = Rc::downgrade(&controller);
        let weak_route = route.downgrade();
        route.retain(true);
        route.on_select(move || {
            if let (Some(controller), Some(route)) = (weak.upgrade(), weak_route.upgrade()) {
                controller.borrow_mut().select(
                    &route,
                    pose.clone(),
                    direction.get(),
                    duration.get(),
                );
            }
        });
    }

    fn select(
        &mut self,
        route: &RoutePresentation,
        pose: LocalProperty<RouteCuboidPose>,
        direction: RouteCuboidDirection,
        duration: Duration,
    ) {
        self.faces.retain(|face| {
            face.route
                .upgrade()
                .is_some_and(|route| route.is_selected() || route.retained_for_presentation())
        });
        route.retain(true);
        if self.faces.is_empty() {
            self.progress.cancel_transitions();
            self.progress.set(1.);
            self.motion = Motion::default();
            self.faces.push(Face {
                route: route.downgrade(),
                orientation: Orientation::default(),
                pose,
            });
            self.present();
            return;
        }

        let progress = self.progress.get();
        let current = self.motion.sample(progress);
        let velocity = self.motion.velocity(progress);
        let existing = self
            .faces
            .iter()
            .position(|face| face.route.upgrade().as_ref() == Some(route));
        let destination = if let Some(index) = existing {
            self.faces[index].orientation.inverse()
        } else {
            let step = quarter(direction);
            let mut destination = step.multiply(self.motion.to);
            // The requested adjacent face may still be visible during very rapid
            // navigation. Continue around that axis to a hidden face, never replace
            // a visible subtree or create an unbounded backlog of route instances.
            for _ in 0..4 {
                if current.multiply(destination.inverse()).normal()[2] <= 1e-7 {
                    break;
                }
                destination = step.multiply(destination);
            }
            let orientation = destination.inverse();
            let normal = orientation.normal();
            self.faces.retain(|face| {
                let other = face.orientation.normal();
                let same_face = (0..3).map(|i| normal[i] * other[i]).sum::<f64>() > 1. - 1e-7;
                if same_face {
                    // Selection can run before this frame's presentation effect.
                    // Close the old aperture even if a child exit delays unmount.
                    let mut pose = face.pose.get();
                    pose.visible = false;
                    face.pose.set(pose);
                    if let Some(route) = face.route.upgrade() {
                        route.retain(false);
                    }
                }
                !same_face
            });
            self.faces.push(Face {
                route: route.downgrade(),
                orientation,
                pose,
            });
            destination
        };
        let millis = duration.as_milliseconds_f64();
        let millis = if millis.is_finite() {
            millis.max(0.)
        } else {
            0.
        };
        self.motion = Motion {
            from: current,
            to: destination,
            tangent: velocity.map(|v| v * millis),
            duration_ms: millis,
        };
        self.progress.cancel_transitions();
        self.progress.set(0.);
        if millis <= 0. {
            self.progress.set(1.);
        } else {
            self.progress.ease_to(1., duration, EasingCurve::Linear);
        }
        self.present();
    }

    fn present(&mut self) {
        let progress = self.progress.get();
        let rotation = self.motion.sample(progress);
        self.faces.retain(|face| face.route.upgrade().is_some());
        for face in &self.faces {
            let orientation = rotation.multiply(face.orientation);
            let [x, y, z] = orientation.normal();
            let [rx, ry, rz] = orientation.euler();
            face.pose.set(RouteCuboidPose {
                x,
                y,
                z,
                rx,
                ry,
                rz,
                visible: z > 1e-7,
            });
        }
        if progress >= 1. {
            self.faces.retain(|face| {
                let keep = face.route.upgrade().is_some_and(|route| {
                    if !route.is_selected() {
                        route.retain(false);
                    }
                    route.is_selected()
                });
                keep
            });
            // Rebase only when all other faces have retired. This keeps later
            // authored turns independent of previous history and numeric drift.
            if let Some(face) = self.faces.first_mut() {
                face.orientation = Orientation::default();
                face.pose.set(RouteCuboidPose {
                    z: 1.,
                    visible: true,
                    ..Default::default()
                });
                self.motion = Motion::default();
            }
        }
    }
}
