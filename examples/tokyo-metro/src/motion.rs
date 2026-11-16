use crate::{
    geometry::{self, Curve, SampledCurve},
    network::Network,
};

const DURATION: u64 = 820;

fn progress(now: u64, started: u64, immediate: bool) -> f64 {
    let t = if immediate {
        1.
    } else {
        (now.saturating_sub(started) as f64 / DURATION as f64).min(1.)
    };
    t * t * (3. - 2. * t)
}

pub struct CameraMotion {
    pub value: f64,
    pub target: f64,
    pub active: bool,
    from: f64,
    started: u64,
}

impl CameraMotion {
    pub fn new(value: f64) -> Self {
        Self {
            value,
            target: value,
            from: value,
            started: 0,
            active: false,
        }
    }
    pub fn retarget(&mut self, target: f64, now: u64) {
        self.from = self.value;
        self.target = target;
        self.started = now;
        self.active = self.value != target;
    }
    pub fn advance(&mut self, now: u64, immediate: bool) -> bool {
        if !self.active {
            return false;
        }
        let t = progress(now, self.started, immediate);
        let previous = self.value;
        self.value = if t == 1. {
            self.target
        } else {
            self.from + (self.target - self.from) * t
        };
        self.active = t < 1.;
        self.value != previous
    }
}

struct MovingLine {
    index: usize,
    from: Vec<Curve>,
    to: Vec<Curve>,
}

pub struct LayoutMotion {
    pub full: Vec<Vec<Curve>>,
    pub current: Vec<Vec<Curve>>,
    pub samples: Vec<Vec<SampledCurve>>,
    pub alpha: Vec<f64>,
    pub size: (f64, f64),
    pub target_size: (f64, f64),
    pub active: bool,
    lines: Vec<MovingLine>,
    alpha_from: Vec<f64>,
    alpha_to: Vec<f64>,
    size_from: (f64, f64),
    started: u64,
    last_progress: f64,
}

impl LayoutMotion {
    pub fn new(full: Vec<Vec<Curve>>) -> Self {
        let count = full.len();
        Self {
            samples: full
                .iter()
                .map(|line| line.iter().copied().map(SampledCurve::new).collect())
                .collect(),
            current: full.clone(),
            full,
            alpha: vec![1.; count],
            size: geometry::MAP_SIZE,
            target_size: geometry::MAP_SIZE,
            active: false,
            lines: vec![],
            alpha_from: vec![1.; count],
            alpha_to: vec![1.; count],
            size_from: geometry::MAP_SIZE,
            started: 0,
            last_progress: 0.,
        }
    }

    pub fn retarget(&mut self, network: &Network, selected: u32, portrait: bool, now: u64) {
        let mut target = self.full.clone();
        self.target_size = geometry::MAP_SIZE;
        self.alpha_from.clone_from(&self.alpha);
        self.alpha_to.fill(if selected == 0 { 1. } else { 0. });
        if let Some(index) = selected.checked_sub(1).map(|i| i as usize) {
            let (curves, size) = geometry::isolated(network, index, portrait);
            target[index] = curves;
            self.target_size = size;
            self.alpha_to[index] = 1.;
        }
        // Retarget from the displayed curves, including a partially completed
        // previous selection. An interrupted A → B transition can move both.
        self.lines = target
            .into_iter()
            .enumerate()
            .filter_map(|(index, to)| {
                (self.current[index] != to).then(|| MovingLine {
                    index,
                    from: self.current[index].clone(),
                    to,
                })
            })
            .collect();
        self.size_from = self.size;
        self.started = now;
        self.last_progress = 0.;
        self.active =
            !self.lines.is_empty() || self.alpha != self.alpha_to || self.size != self.target_size;
    }

    // Return only the lines whose curves and arc-length tables changed.
    pub fn advance(&mut self, now: u64, immediate: bool) -> Vec<usize> {
        if !self.active {
            return vec![];
        }
        let t = progress(now, self.started, immediate);
        if t == self.last_progress {
            return vec![];
        }
        self.last_progress = t;
        let mut changed = Vec::with_capacity(self.lines.len());
        for line in &self.lines {
            let curves = &mut self.current[line.index];
            for ((current, from), to) in curves.iter_mut().zip(&line.from).zip(&line.to) {
                *current = if t == 1. { *to } else { from.mix(to, t) };
            }
            self.samples[line.index] = curves.iter().copied().map(SampledCurve::new).collect();
            changed.push(line.index);
        }
        for ((alpha, from), to) in self
            .alpha
            .iter_mut()
            .zip(&self.alpha_from)
            .zip(&self.alpha_to)
        {
            *alpha = if t == 1. { *to } else { from + (to - from) * t };
        }
        self.size = if t == 1. {
            self.target_size
        } else {
            (
                self.size_from.0 + (self.target_size.0 - self.size_from.0) * t,
                self.size_from.1 + (self.target_size.1 - self.size_from.1) * t,
            )
        };
        self.active = t < 1.;
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn zoom_retarget_does_not_touch_geometry_or_restart_a_morph() {
        let n = Network::tokyo();
        let mut layout = LayoutMotion::new(geometry::map(&n));
        let mut camera = CameraMotion::new(0.4);
        let untouched = layout.samples[0].as_ptr();
        layout.retarget(&n, 14, false, 100);
        assert_eq!(layout.advance(300, false), vec![13]);
        camera.retarget(0.8, 300);
        camera.advance(500, false);
        camera.retarget(0.6, 500);
        assert_eq!(layout.advance(920, false), vec![13]);
        assert!(!layout.active);
        assert_eq!(layout.samples[0].as_ptr(), untouched);
        assert_eq!(layout.current[0], layout.full[0]);
        assert_eq!(layout.current[13], geometry::isolated(&n, 13, false).0);
        assert!(layout.advance(1000, false).is_empty());
    }
    #[test]
    fn interrupted_selection_updates_only_the_old_and_new_lines() {
        let n = Network::tokyo();
        let mut layout = LayoutMotion::new(geometry::map(&n));
        layout.retarget(&n, 1, false, 100);
        assert_eq!(layout.advance(400, false), vec![0]);
        let displayed = layout.current.clone();
        layout.retarget(&n, 2, true, 400);
        assert!(layout.advance(400, false).is_empty());
        assert_eq!(layout.current, displayed);
        assert_eq!(layout.advance(1220, false), vec![0, 1]);
        assert_eq!(layout.current[0], layout.full[0]);
        assert_eq!(layout.current[1], geometry::isolated(&n, 1, true).0);
        layout.retarget(&n, 0, true, 1300);
        assert_eq!(layout.advance(1300, true), vec![1]);
        assert_eq!(layout.current, layout.full);
        assert!(layout.alpha.iter().all(|a| *a == 1.));
    }
    #[test]
    fn orientation_and_reduced_motion_keep_exact_endpoints() {
        let n = Network::tokyo();
        let mut layout = LayoutMotion::new(geometry::map(&n));
        layout.retarget(&n, 13, false, 0);
        layout.advance(0, true);
        layout.retarget(&n, 13, true, 10);
        assert_eq!(layout.advance(20, false), vec![12]);
        assert_eq!(layout.advance(20, true), vec![12]);
        assert_eq!(layout.current[12], geometry::isolated(&n, 12, true).0);
        let mut camera = CameraMotion::new(0.4);
        camera.retarget(0.8, 10);
        camera.advance(200, false);
        let displayed = camera.value;
        camera.retarget(0.3, 200);
        assert!(!camera.advance(200, false));
        assert_eq!(camera.value, displayed);
        camera.advance(200, true);
        assert_eq!(camera.value, 0.3);
        assert!(!camera.active);
    }
}
