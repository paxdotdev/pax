use super::{
    activity::{Activity, Gesture},
    indexed::{Indexed, Next},
    reference::Reference,
    *,
};
fn h(id: Id, parent: Domain, b: Rect, axis: usize, offset: f64) -> Host {
    let viewport = [b.width(), b.height()];
    let mut content = viewport;
    content[axis] = 20000.;
    let mut off = [0.; 2];
    off[axis] = offset;
    Host {
        id,
        parent,
        bounds: b,
        clip: None,
        viewport,
        content,
        offset: off,
        origin: [0.; 2],
        basis: [1., 0., 0., 1.],
        travel: [0.; 2],
    }
}
fn base() -> Scene {
    let mut s = Scene::new(rect([0., 0., 100., 100.]));
    s.active = Some(1);
    s.direction = [0, 1];
    for host in [
        h(1, None, s.viewport, 1, 500.),
        h(2, Some(1), rect([10., 130., 90., 170.]), 0, 100.),
        h(3, Some(1), rect([5., 20., 55., 60.]), 0, 100.),
    ] {
        s.hosts.insert(host.id, host);
    }
    s
}
fn box_at(x: f64, y: f64) -> Rect {
    Rect::new(x, y, x + 8., y + 8.)
}
struct Rig {
    scene: Scene,
    fast: Indexed,
    owners: BTreeMap<Id, Owner>,
    jobs: BTreeMap<Id, Job>,
    meter: Service,
}
impl Rig {
    fn new(s: Scene) -> Self {
        Self {
            fast: Indexed::new(s.clone()),
            scene: s,
            owners: Default::default(),
            jobs: Default::default(),
            meter: Default::default(),
        }
    }
    fn owner(&mut self, id: Id, domain: Domain, screen: Option<Rect>) {
        let o = Owner {
            domain,
            bounds: screen.map(|b| b + self.scene.bias(domain)),
            epoch: 1,
        };
        self.owners.insert(id, o.clone());
        self.fast.put_owner(id, o);
    }
    fn job(&mut self, id: Id, owners: &[Id], resource: Resource, status: Status) {
        let j = Job {
            age: id,
            owners: owners.to_vec(),
            lifetime: 1,
            content: 1,
            resource,
            status,
        };
        self.update(id, j);
    }
    fn update(&mut self, id: Id, j: Job) {
        self.jobs.insert(id, j.clone());
        self.fast.put_job(id, j);
    }
    fn delete(&mut self, id: Id) {
        self.jobs.remove(&id);
        self.fast.remove_job(id);
    }
    fn entry(&mut self, id: Id, dom: Id, x: f64, y: f64) {
        self.owner(id, Some(dom), Some(box_at(x, y)));
        self.job(id, &[id], 0, Status::Runnable);
    }
    fn observe(&mut self, s: Scene) {
        self.fast.observe(s.clone());
        self.scene = s;
        self.meter.observe(&self.scene);
    }
    fn check(&mut self, resource: Resource) -> Option<Choice> {
        self.fast.settle(usize::MAX);
        let all =
            Reference::new(&self.scene).enumerate(&self.jobs, &self.owners, resource, &self.meter);
        let want = all.values().min_by_key(|c| self.meter.order(**c)).copied();
        assert_eq!(self.fast.next(resource, &self.meter), Next::Selected(want));
        let work = (
            self.fast.stats.classifications,
            self.fast.stats.cold_candidates,
        );
        assert_eq!(self.fast.next(resource, &self.meter), Next::Selected(want));
        assert_eq!(
            work,
            (
                self.fast.stats.classifications,
                self.fast.stats.cold_candidates
            )
        );
        want
    }
    fn expect(&mut self, id: Id, tier: Tier) {
        let c = self.check(0).unwrap();
        assert_eq!((c.id, c.tier), (id, tier));
    }
}
#[test]
fn clipped_nested_bands_donation_and_partial_discovery() {
    let mut r = Rig::new(base());
    r.entry(20, 2, 20., 139.); // current shelf column, reached by page scroll alone
    r.entry(21, 2, 108., 139.); // needs page and horizontal shelf scroll
    r.entry(22, 3, 66., 35.); // needs only the currently visible branch to scroll
    r.entry(23, 1, 30., -65.);
    r.entry(24, 1, 30., 260.);
    r.expect(20, Tier::Leading);
    r.delete(20);
    r.expect(23, Tier::OtherWarm);
    r.delete(23);
    r.expect(22, Tier::NestedVisible);
    r.delete(22);
    r.expect(21, Tier::NestedWarm);
    r.delete(21);
    r.expect(24, Tier::Forward);
    r.owner(50, Some(3), Some(box_at(25., 25.)));
    // Discovery can run on the nearest mounted anchor before the slot has a footprint.
    r.owner(51, Some(2), None);
    r.job(11, &[51, 50], 0, Status::Runnable);
    r.job(12, &[24, 50], 0, Status::Runnable);
    r.expect(11, Tier::Visible);
    r.delete(11);
    r.expect(12, Tier::Visible);
    r.job(12, &[24], 0, Status::Runnable);
    r.expect(12, Tier::Forward);
}
#[test]
fn queued_extent_follows_two_later_rows_through_a_jump_and_reversal() {
    let mut r = Rig::new(base());
    r.owner(10, Some(1), Some(box_at(10., 900.)));
    r.owner(11, Some(1), Some(box_at(10., 220.)));
    r.owner(12, Some(1), Some(box_at(10., 520.)));
    r.job(1, &[10, 11, 12], 0, Status::Runnable);
    r.entry(3, 1, 10., 15.);
    let home = r.scene.clone();
    r.expect(3, Tier::Visible);
    for offset in [200., 500., 200.] {
        let mut next = home.clone();
        next.hosts.get_mut(&1).unwrap().offset[1] += offset;
        r.observe(next);
        r.expect(1, Tier::Visible);
    }
    r.observe(home);
    r.expect(3, Tier::Visible);
    r.delete(3);
    assert_eq!(r.check(0).unwrap().id, 1);
}
#[test]
fn previous_loser_promotes_on_jump_and_demotes_on_reversal() {
    let mut r = Rig::new(base());
    r.entry(5, 1, 10., 10.);
    r.entry(6, 1, 10., 530.);
    r.entry(7, 1, 10., 470.);
    r.entry(8, 1, 10., 607.);
    r.expect(5, Tier::Visible);
    let mut s = r.scene.clone();
    s.hosts.get_mut(&1).unwrap().offset[1] += 500.;
    for h in [2, 3] {
        let host = s.hosts.get_mut(&h).unwrap();
        host.bounds = host.bounds + Vec2::new(0., -500.);
    }
    r.observe(s);
    assert_eq!(r.fast.next(0, &r.meter), Next::Refresh);
    r.fast.settle(1);
    assert_eq!(r.fast.next(0, &r.meter), Next::Refresh);
    r.expect(6, Tier::Visible);
    r.delete(6);
    r.expect(8, Tier::Leading);
    let mut s = r.scene.clone();
    s.direction = [0, -1];
    r.observe(s);
    r.expect(7, Tier::Leading);
    let mut s = r.scene.clone();
    s.direction = [0, 0];
    r.observe(s);
    r.expect(8, Tier::OtherWarm);
}
#[test]
fn intermediate_clips_and_teardown_recompute_donations() {
    let mut r = Rig::new(base());
    r.entry(9, 3, 14., 25.);
    r.entry(10, 1, 20., 110.);
    r.expect(9, Tier::Visible);
    let mut s = r.scene.clone();
    s.hosts.get_mut(&3).unwrap().clip = Some(box_at(90., 80.));
    r.observe(s);
    r.expect(10, Tier::Leading);
    let mut s = r.scene.clone();
    s.hosts.remove(&3);
    r.observe(s);
    r.expect(10, Tier::Leading);
    let mut s = r.scene.clone();
    s.hosts
        .insert(3, h(3, Some(1), rect([5., 20., 55., 60.]), 0, 100.));
    r.observe(s);
    r.expect(9, Tier::Visible);
}
#[test]
fn observed_diagonal_requires_one_common_instant() {
    let mut s = base();
    s.active = None;
    s.direction = [0, 0];
    s.safety = 0.;
    s.reverse = 0.;
    s.hosts.get_mut(&2).unwrap().bounds = rect([10., 105., 50., 145.]);
    s.hosts.get_mut(&2).unwrap().travel = [80., 0.];
    s.hosts.get_mut(&1).unwrap().travel = [0., 100.];
    let mut r = Rig::new(s);
    r.entry(2, 2, 62., 118.);
    r.entry(1, 2, 115., 198.);
    r.expect(2, Tier::Leading);
    r.delete(2);
    r.expect(1, Tier::Remote);
    // The parent can reach the shelf only near the end. Its exposed columns
    // have already moved, so the envelope corner is impossible.
    let mut s = r.scene.clone();
    s.hosts.get_mut(&2).unwrap().bounds = rect([10., 181., 50., 221.]);
    r.observe(s);
    r.delete(1);
    r.entry(3, 2, 10., 183.);
    r.expect(3, Tier::Remote);
}
#[test]
fn physical_ownership_momentum_autoplay_recenter_and_expiry() {
    let mut s = base();
    let mut a = Activity::with_tail(100.);
    let mut pos: BTreeMap<_, _> = s.hosts.iter().map(|(&i, h)| (i, h.offset)).collect();
    a.update(&s, &pos, &[], 0.);
    let mut observe = |s: &Scene, p: &BTreeMap<Id, [f64; 2]>, g: &[Gesture], t: f64| {
        a.update(s, p, g, t);
        a.project(s.clone(), t, 200.)
    };
    pos.get_mut(&1).unwrap()[1] += 20.;
    let next = observe(
        &s,
        &pos,
        &[Gesture {
            host: Some(1),
            delta: [0., 20.],
        }],
        10.,
    );
    assert_eq!(next.active, Some(1));
    assert_eq!(next.direction, [0, 1]);
    pos.get_mut(&2).unwrap()[0] += 9.;
    pos.get_mut(&1).unwrap()[1] += 10.;
    let next = observe(&s, &pos, &[], 30.);
    assert_eq!(next.active, Some(1));
    assert_eq!(next.hosts[&2].travel, [0., 0.]);
    assert!(next.hosts[&1].travel[1] > 0.);
    pos.get_mut(&2).unwrap()[0] -= 12.;
    let next = observe(
        &s,
        &pos,
        &[Gesture {
            host: Some(2),
            delta: [-12., 0.],
        }],
        40.,
    );
    assert_eq!(next.active, Some(2));
    assert_eq!(next.direction, [-1, 0]);
    assert!(next.hosts[&1].travel[1] > 0.);
    pos.get_mut(&2).unwrap()[0] += 450.;
    s.hosts.get_mut(&2).unwrap().origin[0] -= 300.;
    let next = observe(&s, &pos, &[], 50.);
    assert_eq!(next.hosts[&2].travel, [0., 0.]);
    let next = observe(&s, &pos, &[], 200.);
    assert_eq!(next.direction, [0, 0]);
}
#[test]
fn blocked_work_gpu_revisions_and_runtime_branch_replacement() {
    let mut r = Rig::new(base());
    r.owner(1, Some(3), Some(box_at(17., 26.)));
    r.owner(2, Some(1), Some(box_at(25., 300.)));
    r.job(10, &[1], 0, Status::Waiting(Wake::Measurement(81)));
    r.job(11, &[1], 1, Status::Submitted(81));
    r.job(12, &[2], 0, Status::Runnable);
    r.expect(12, Tier::Forward);
    assert!(r.check(1).is_none());
    let mut measurement = r.jobs[&10].clone();
    assert!(!measurement.wake(1, 1, Wake::Measurement(80)));
    assert!(measurement.wake(1, 1, Wake::Measurement(81)));
    r.update(10, measurement);
    r.expect(10, Tier::Visible);
    // The conditional now chooses a different branch. The previous submitted
    // identity cannot publish into the reused node even if the token matches.
    let mut gpu = r.jobs[&11].clone();
    gpu.status = Status::Retired;
    r.update(11, gpu.clone());
    assert!(!gpu.wake(1, 1, Wake::Fence(81)));
    gpu.lifetime += 1;
    gpu.content += 1;
    gpu.status = Status::Submitted(81);
    assert!(!gpu.wake(1, 2, Wake::Fence(81)));
    assert!(!gpu.wake(2, 1, Wake::Fence(81)));
    assert!(gpu.wake(2, 2, Wake::Fence(81)));
    assert_eq!(gpu.status, Status::Ready);
    r.delete(10);
    r.job(13, &[1], 0, Status::Failed(Failure::MissingProducer));
    r.expect(12, Tier::Forward);
    r.delete(12);
    assert!(r.check(0).is_none());
}
#[test]
fn service_is_charged_in_microseconds_and_idle_work_drains() {
    let mut r = Rig::new(base());
    r.entry(4, 1, 20., 400.);
    r.entry(5, 1, 20., -300.);
    let mut total = [0; 2];
    for tick in 0..120 {
        let c = r.check(0).unwrap();
        let n = [100, 200, 300][tick % 3];
        let lane = usize::from(c.tier == Tier::Reverse);
        total[lane] += n;
        r.meter.charge(c.tier, c.host, n);
    }
    assert!((total[0] as i64 - 2 * total[1] as i64).abs() <= 600);
    r.meter = Service::new([1, 1], 2000);
    let mut counts = [0; 2];
    for _ in 0..20 {
        let c = r.check(0).unwrap();
        counts[usize::from(c.tier == Tier::Reverse)] += 1;
        r.meter.charge(c.tier, c.host, 50);
    }
    assert_eq!(counts, [10, 10]);
    r.entry(12, 2, 500., 150.);
    r.entry(13, 3, 600., 24.);
    r.entry(14, 3, 450., 24.);
    let mut served = Vec::new();
    while let Some(c) = r.check(0) {
        r.meter.charge(c.tier, c.host, 100);
        served.push((c.id, c.host));
        r.delete(c.id);
    }
    assert_eq!(served.len(), 5);
    assert_ne!(served[2].1, served[3].1);
}
#[test]
fn small_scroll_and_unchanged_yields_are_local_in_a_large_queue() {
    let mut s = base();
    s.active = Some(3);
    s.direction = [1, 0];
    let mut r = Rig::new(s);
    for i in 0..4096 {
        r.entry(100 + i, 1, 20., 1500. + i as f64 * 90.);
    }
    r.entry(1, 3, 22., 35.);
    r.entry(2, 3, 72., 35.);
    r.expect(1, Tier::Visible);
    let before = r.fast.stats.classifications;
    let mut s = r.scene.clone();
    s.hosts.get_mut(&3).unwrap().offset[0] += 32.;
    r.observe(s);
    r.expect(2, Tier::Visible);
    let after = r.fast.stats.classifications;
    assert!(after - before <= 2, "{}", after - before);
    for _ in 0..12 {
        assert!(matches!(r.fast.next(0, &r.meter), Next::Selected(Some(_))));
    }
    assert_eq!(r.fast.stats.classifications, after);
    eprintln!(
        "local scroll: {} owner classifications among {} owners",
        after - before,
        r.owners.len()
    );
}
#[test]
fn arbitrary_visible_depth_and_bounded_speculation() {
    let mut s = base();
    for i in 4..22 {
        s.hosts.insert(
            i,
            h(
                i,
                Some(if i == 4 { 3 } else { i - 1 }),
                rect([8., 24., 44., 55.]),
                0,
                0.,
            ),
        );
    }
    let mut r = Rig::new(s);
    r.entry(1, 21, 19., 29.);
    r.expect(1, Tier::Visible);
}
#[test]
fn incomplete_slices_and_stale_completions_keep_a_single_ticket() {
    let mut r = Rig::new(base());
    r.entry(2, 1, 5., 50.);
    r.entry(3, 1, 5., 175.);
    let mut cursor = BTreeMap::from([(2, 0), (3, 0)]);
    let mut serve = |r: &mut Rig| {
        let c = r.check(0).unwrap();
        let count = cursor.get_mut(&c.id).unwrap();
        *count += 1;
        (c.id, *count)
    };
    assert_eq!(serve(&mut r), (2, 1));
    let mut scene = r.scene.clone();
    scene.hosts.get_mut(&1).unwrap().offset[1] += 150.;
    r.observe(scene);
    assert_eq!(serve(&mut r), (3, 1));
    let mut scene = r.scene.clone();
    scene.hosts.get_mut(&1).unwrap().offset[1] -= 150.;
    r.observe(scene);
    assert_eq!(serve(&mut r), (2, 2));
    r.job(4, &[2], 1, Status::Waiting(Wake::Settlement(42)));
    assert!(!r.fast.signal(4, 1, 1, Wake::Fence(42)));
    assert!(!r.fast.signal(4, 99, 1, Wake::Settlement(42)));
    assert!(r.fast.signal(4, 1, 1, Wake::Settlement(42)));
    assert_eq!(r.fast.outcome(4), Some(Status::Runnable));
    r.job(5, &[2], 2, Status::Failed(Failure::MissingProducer));
    assert_eq!(
        r.fast.outcome(5),
        Some(Status::Failed(Failure::MissingProducer))
    );
    r.job(6, &[], 2, Status::Runnable);
    assert_eq!(
        r.fast.outcome(6),
        Some(Status::Failed(Failure::MissingOwner))
    );
}
#[test]
fn many_transformed_and_clipped_routes_agree_owner_by_owner() {
    let mut seed = 127u64;
    let mut n = |max| {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        ((seed >> 32) % (max as u64)) as f64
    };
    for _ in 0..120 {
        let mut s = base();
        s.direction = [0, if n(2) == 0. { -1 } else { 1 }];
        let y = n(180) - 40.;
        let sh = s.hosts.get_mut(&2).unwrap();
        sh.bounds = rect([8., y, 88., y + 40.]);
        sh.basis = [0.85, 0.25, -0.25, 0.85];
        sh.clip = Some(rect([n(30), y + 3., 82., y + 32.]));
        sh.travel = [n(110) - 55., 0.];
        s.hosts.get_mut(&1).unwrap().travel = [0., n(160) - 60.];
        let plan = super::regions::Plan::build(&s, Some(2));
        let mut slow = Reference::new(&s);
        for _ in 0..60 {
            let b = box_at(n(230) - 65., n(240) - 70.) + s.bias(Some(2));
            let o = Owner {
                bounds: Some(b),
                epoch: 1,
                domain: Some(2),
            };
            assert_eq!(
                plan.tier(b),
                slow.owner(&o).0,
                "screen bounds {:?}",
                b - s.bias(Some(2))
            );
        }
    }
}
#[test]
fn pending_remote_index_work_cannot_hold_an_already_proven_visible_winner() {
    let mut r = Rig::new(base());
    r.entry(1, 1, 2., 20.);
    r.expect(1, Tier::Visible);
    r.entry(2, 1, 20., 1300.);
    assert!(matches!(r.fast.next(0,&r.meter),Next::Selected(Some(c)) if c.id==1));
    r.owner(2, Some(1), Some(box_at(20., 22.)));
    assert_eq!(r.fast.next(0, &r.meter), Next::Refresh);
    r.fast.settle(1);
    r.expect(1, Tier::Visible);
}
