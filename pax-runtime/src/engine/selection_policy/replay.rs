use super::{
    activity::{Activity, Gesture},
    indexed::{Indexed, Next},
    reference::Reference,
    *,
};
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawHost {
    id: Id,
    parent: Domain,
    bounds: [f64; 4],
    viewport: [f64; 2],
    offset: [f64; 2],
    observed: [f64; 2],
    basis: [f64; 4],
    content: [f64; 2],
    content_origin: [f64; 2],
    intermediate_clip: Option<[f64; 4]>,
}
#[derive(Deserialize)]
struct Observation {
    serial: u32,
    at: f64,
    input: u32,
    viewport: [f64; 4],
    hosts: Vec<RawHost>,
    horizon: f64,
}
#[derive(Deserialize)]
struct Input {
    serial: u32,
    host: Domain,
    delta: [f64; 2],
}
#[derive(Clone, Deserialize, serde::Serialize)]
struct Obligation {
    age: Id,
    #[serde(default)]
    priority_age: Option<Id>,
    life: (u32, u64, u32, u64),
    owner: Id,
    group: Domain,
    kind: String,
    component: String,
}
#[derive(Clone, Deserialize)]
struct Footprint {
    id: Id,
    epoch: u64,
    host: Domain,
    bounds: Option<[f64; 4]>,
    clip: Option<[f64; 4]>,
}
#[derive(Deserialize)]
struct Decision {
    at: f64,
    observation: u32,
    add: Vec<Obligation>,
    remove: Vec<Id>,
    geometry: Vec<Footprint>,
    signal_geometry: Option<Vec<Footprint>>,
    discarded: Option<String>,
    eligible: usize,
    selected: Option<Id>,
}
#[derive(Deserialize)]
struct Captured {
    #[serde(default)]
    signal_reads: u64,
    #[serde(default)]
    full_reads: u64,
    #[serde(default)]
    dependency_overflows: u32,
    #[serde(default)]
    signal_disagreements: u32,
    choices: Vec<Decision>,
    receipts: Vec<(f64, Id)>,
}
#[derive(Deserialize)]
struct Selection {
    observations: Vec<Observation>,
    inputs: Vec<Input>,
    report: Captured,
}
#[derive(Deserialize)]
struct Trace {
    grants: Vec<Value>,
    groups: Vec<Value>,
}
#[derive(Deserialize)]
struct Page {
    selection: Selection,
    trace: Trace,
    label: String,
}
fn decode_obs(o: &Observation) -> (Scene, BTreeMap<Id, [f64; 2]>) {
    let mut s = Scene::new(rect(o.viewport));
    let mut samples = BTreeMap::new();
    for h in &o.hosts {
        s.hosts.insert(
            h.id,
            Host {
                id: h.id,
                parent: h.parent,
                bounds: rect(h.bounds),
                clip: h.intermediate_clip.map(rect),
                viewport: h.viewport,
                content: h.content,
                origin: h.content_origin,
                offset: h.offset,
                basis: h.basis,
                travel: [0., 0.],
            },
        );
        samples.insert(h.id, h.observed);
    }
    (s, samples)
}
// Oracle snapshots may enumerate the whole frontier. The incremental feed uses
// only work membership, signalled owner changes and changed coordinate bases.
#[derive(Default)]
struct Feed {
    counts: BTreeMap<Id, usize>,
    inputs: BTreeMap<Id, Footprint>,
    domains: BTreeMap<Domain, std::collections::BTreeSet<Id>>,
    converted: u64,
}
impl Feed {
    fn release(&mut self, id: Id, fast: &mut Indexed) {
        let n = self.counts.get_mut(&id).expect("known admission owner");
        *n -= 1;
        if *n != 0 {
            return;
        }
        self.counts.remove(&id);
        if let Some(old) = self.inputs.remove(&id) {
            if let Some(ids) = self.domains.get_mut(&old.host) {
                ids.remove(&id);
                if ids.is_empty() {
                    self.domains.remove(&old.host);
                }
            }
        }
        fast.remove_owner(id);
    }
    fn update(
        &mut self,
        changes: &[Footprint],
        scene: &Scene,
        fast: &mut Indexed,
    ) -> Result<(), String> {
        let mut dirty = std::collections::BTreeSet::new();
        for (&dom, ids) in &self.domains {
            if scene.bias(dom) != fast.scene.bias(dom) {
                dirty.extend(ids.iter().copied());
            }
        }
        for f in changes {
            if !self.counts.contains_key(&f.id) {
                continue;
            }
            if let Some(old) = self.inputs.insert(f.id, f.clone()) {
                if old.host != f.host {
                    if let Some(ids) = self.domains.get_mut(&old.host) {
                        ids.remove(&f.id);
                        if ids.is_empty() {
                            self.domains.remove(&old.host);
                        }
                    }
                }
            }
            self.domains.entry(f.host).or_default().insert(f.id);
            dirty.insert(f.id);
        }
        fast.observe(scene.clone());
        for id in dirty {
            let f = &self.inputs[&id];
            let b = f
                .bounds
                .map(rect)
                .map(|b| f.clip.map_or(b, |c| b.intersect(rect(c))))
                .filter(|r| area(*r));
            fast.put_owner(
                id,
                Owner {
                    domain: f.host,
                    bounds: b.map(|b| b + scene.bias(f.host)),
                    epoch: f.epoch,
                },
            );
            self.converted += 1;
        }
        Ok(())
    }
}
pub(super) fn analyze(input: &str) -> Result<Value, String> {
    let page: Page = serde_json::from_str(input).map_err(|e| e.to_string())?;
    let src = page.selection;
    let choices = src.report.choices;
    if choices.is_empty() {
        return Err("empty capture".into());
    }
    let actual_times: BTreeMap<Id, f64> = src
        .report
        .receipts
        .iter()
        .map(|&(time, id)| (id, time))
        .collect();
    let mut feed = Feed::default();
    let mut old_discards = BTreeMap::<String, usize>::new();
    let mut mismatch_count = 0;
    let mut actual_discovery: BTreeMap<Id, (f64, Id, String)> = BTreeMap::new();
    for d in &choices {
        for j in &d.add {
            actual_discovery
                .entry(j.life.0)
                .or_insert((d.at, j.life.2, j.kind.clone()));
        }
    }
    let group_times: BTreeMap<Id, f64> = page
        .trace
        .groups
        .iter()
        .filter_map(|g| Some((g[1].as_u64()? as Id, g[0].as_f64()?)))
        .collect();
    let grants: Vec<_> = page
        .trace
        .grants
        .iter()
        .filter_map(|g| Some((g[0].as_f64()?, g[0].as_f64()? + g[1].as_f64()?)))
        .collect();
    let mut at_grant = 0;
    let mut meter = Service::default();
    let mut base = Scene::new(Rect::ZERO);
    let mut index = Indexed::new(base.clone());
    let mut tracker = Activity::default();
    let (mut oi, mut ii, mut serial, mut horizon) = (0, 0, 0, 250.);
    let mut signal_history = BTreeMap::<Id, Footprint>::new();
    let mut raw = BTreeMap::<Id, Footprint>::new();
    let mut owners = BTreeMap::new();
    let mut jobs = BTreeMap::new();
    let mut meta = BTreeMap::new();
    let (
        mut compared,
        mut eligible,
        mut matching_old,
        mut stale,
        mut ties,
        mut repeated,
        mut rank_full,
        mut measured_charges,
    ) = (0, 0, 0, 0, 0, 0, 0, 0);
    let mut differences = Vec::new();
    let mut inversions = Vec::new();
    let mut mismatch = Vec::new();
    let mut tiers = BTreeMap::<String, u32>::new();
    for (step, d) in choices.iter().enumerate() {
        while oi < src.observations.len() && src.observations[oi].serial <= d.observation {
            let o = &src.observations[oi];
            let (s, positions) = decode_obs(o);
            let mut cues = Vec::new();
            while ii < src.inputs.len() && src.inputs[ii].serial <= o.input {
                let i = &src.inputs[ii];
                cues.push(Gesture {
                    host: i.host,
                    delta: i.delta,
                });
                ii += 1;
            }
            tracker.update(&s, &positions, &cues, o.at);
            base = s;
            horizon = o.horizon;
            serial = o.serial;
            oi += 1;
        }
        if serial != d.observation {
            return Err(format!("missing host observation {}", d.observation));
        }
        let scene = tracker.project(base.clone(), d.at, horizon);
        meter.observe(&scene);
        for o in &d.geometry {
            raw.insert(o.id, o.clone());
        }
        for o in d.signal_geometry.iter().flatten() {
            signal_history.insert(o.id, o.clone());
        }
        for id in &d.remove {
            jobs.remove(id);
            if let Some(j) = meta.remove(id) {
                let j: Obligation = j;
                feed.release(j.owner, &mut index);
            }
            index.remove_job(*id);
        }
        for j in &d.add {
            let job = Job {
                age: j.priority_age.unwrap_or(j.age),
                owners: vec![j.owner],
                lifetime: j.life.3,
                content: 0,
                resource: 0,
                status: Status::Runnable,
            };
            let prior = meta.get(&j.age).map(|old: &Obligation| old.owner);
            if prior != Some(j.owner) {
                if let Some(old) = prior {
                    feed.release(old, &mut index);
                }
                *feed.counts.entry(j.owner).or_default() += 1;
            }
            jobs.insert(j.age, job.clone());
            meta.insert(j.age, j.clone());
            index.put_job(j.age, job);
        }
        // A remove and add can transfer the last subscriber to another job in
        // one observation. The owner then keeps its raw footprint and lifetime.
        let mut changes = d.signal_geometry.as_ref().unwrap_or(&d.geometry).clone();
        for j in &d.add {
            if !feed.inputs.contains_key(&j.owner) && !changes.iter().any(|f| f.id == j.owner) {
                let f = if d.signal_geometry.is_some() {
                    signal_history.get(&j.owner)
                } else {
                    raw.get(&j.owner)
                };
                if let Some(f) = f {
                    changes.push(f.clone());
                } else {
                    return Err(format!("missing subscribed input for {}", j.owner));
                }
            }
        }
        feed.update(&changes, &scene, &mut index)?;
        let live: std::collections::BTreeSet<Id> = jobs
            .values()
            .flat_map(|j| j.owners.iter().copied())
            .collect();
        let old: Vec<_> = owners
            .keys()
            .copied()
            .filter(|id| !live.contains(id))
            .collect();
        for id in old {
            owners.remove(&id);
        }
        for id in &live {
            let f = raw.get(id).ok_or_else(|| format!("missing owner {id}"))?;
            let b = f
                .bounds
                .map(rect)
                .map(|b| f.clip.map_or(b, |c| b.intersect(rect(c))))
                .filter(|r| area(*r));
            let o = Owner {
                domain: f.host,
                bounds: b.map(|b| b + scene.bias(f.host)),
                epoch: f.epoch,
            };
            owners.insert(*id, o);
        }
        if jobs.len() != d.eligible {
            return Err(format!(
                "choice {step} captured {} of {}",
                jobs.len(),
                d.eligible
            ));
        }
        index.settle(usize::MAX);
        let Next::Selected(proposed) = index.next(0, &meter) else {
            return Err("unfinished refresh".into());
        };
        let before = (index.stats.classifications, index.stats.cold_candidates);
        if index.next(0, &meter) != Next::Selected(proposed)
            || (index.stats.classifications, index.stats.cold_candidates) != before
        {
            return Err("unchanged resumption performed work".into());
        }
        repeated += 1;
        let mut oracle = Reference::new(&scene);
        let all = oracle.enumerate(&jobs, &owners, 0, &meter);
        if all.len() != d.eligible {
            return Err(format!("choice {step} has unrated live work"));
        }
        let want = all.values().min_by_key(|c| meter.order(**c)).copied();
        if proposed != want {
            mismatch_count += 1;
            if mismatch.len() < 12 {
                mismatch.push(json!({"decision":step,"at":d.at,"indexed":proposed,"reference":want,"old":d.selected}));
            }
        }
        eligible += all.len();
        rank_full += live.len();
        compared += 1;
        if let Some(w) = want {
            *tiers.entry(format!("{:?}", w.tier)).or_default() += 1;
        }
        if want.map(|c| c.id) == d.selected {
            matching_old += 1;
        }
        if let Some(c) = d.selected.and_then(|id| all.get(&id).copied()) {
            if let Some(w) = want.filter(|w| w.id != c.id) {
                let duration = actual_times.get(&w.id).map(|v| *v - d.at);
                let evidence = || {
                    json!({"decision":step,"at":d.at,"old":c,"winner":w,"eligible":d.eligible,
                    "winnerJob":meta[&w.id],"oldJob":meta[&c.id],"winnerWaitMs":duration,
                    "firstObservedChild":actual_discovery.get(&meta[&w.id].life.2),
                    "groupOpenedAt":meta[&w.id].group.and_then(|id|group_times.get(&id).copied())})
                };
                if differences.len() < 3 {
                    differences.push(evidence());
                }
                if c.tier != w.tier {
                    inversions.push(evidence());
                } else {
                    ties += 1;
                }
            }
            // Charge only time actually inside the enclosing mount grant after
            // old-policy selection. The next checked frontier ends this unit.
            if let Some(&start) = actual_times.get(&c.id) {
                while at_grant < grants.len() && grants[at_grant].1 + 0.11 < start {
                    at_grant += 1;
                }
                let charged = grants
                    .get(at_grant)
                    .filter(|g| g.0 <= start + 0.11)
                    .map(|g| {
                        (g.1.min(choices.get(step + 1).map_or(g.1, |n| n.at)) - start).max(0.)
                    });
                if charged.is_some() {
                    measured_charges += 1;
                }
                meter.charge(
                    c.tier,
                    c.host,
                    charged.map_or(1, |ms| (ms * 1000.).round().max(1.) as u32),
                );
            }
        } else if d.selected.is_some() {
            stale += 1;
            *old_discards
                .entry(
                    d.discarded
                        .clone()
                        .unwrap_or_else(|| "unavailable in first recording".into()),
                )
                .or_default() += 1;
        }
    }
    Ok(
        json!({"label":page.label,"choices":compared,"eligibleComparisons":eligible,"fullOwnerRanks":rank_full,"indexed":index.stats,
              "unchangedYields":repeated,"oldExact":matching_old,"oldOutsideSnapshot":stale,"sameTierOtherChoice":ties,
              "measuredServiceWindows":measured_charges,"winnersByTier":tiers,"firstDifferences":differences,"tierDifferences":inversions,
              "discardReasons":old_discards,"indexedGeometryConversions":feed.converted,"mismatchCount":mismatch_count,
              "retainedReads":src.report.signal_reads,"exhaustiveReads":src.report.full_reads,"geometryDifferences":src.report.signal_disagreements,
              "dependencyOverflows":src.report.dependency_overflows,"mismatches":mismatch}),
    )
}
#[cfg(test)]
#[test]
fn captured_homepage_frontiers() {
    if let Ok(path) = std::env::var("PAX_SELECTION_INPUT") {
        let input = std::fs::read_to_string(path).unwrap();
        let result = analyze(&input).unwrap();
        std::fs::write(
            std::env::var("PAX_SELECTION_RESULT").unwrap(),
            serde_json::to_vec_pretty(&result).unwrap(),
        )
        .unwrap();
        assert_eq!(result["mismatches"], json!([]));
        assert_eq!(result["geometryDifferences"], 0);
    }
}
