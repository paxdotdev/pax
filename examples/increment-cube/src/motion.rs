const SETTLE_MS: f64 = 1000.0;
const TURN: f64 = 360.0;

// Inverse face transforms: each numbered face ends upright toward the viewer.
// Order matches CuboidSide: front, back, right, left, top, bottom.
pub const REST_POSES: [[f64; 3]; 6] = [
    [0.0, 0.0, 0.0],
    [0.0, 180.0, 0.0],
    [0.0, -90.0, 0.0],
    [0.0, 90.0, 0.0],
    [-90.0, 0.0, 0.0],
    [90.0, 0.0, 0.0],
];

// Z components of the six outward normals after X/Y/Z rotation. Z rotation
// leaves these components unchanged in an orthographic view.
pub fn facing([x, y, _]: [f64; 3]) -> [f64; 6] {
    let (sx, cx) = x.to_radians().sin_cos();
    let (sy, cy) = y.to_radians().sin_cos();
    [cx * cy, -cx * cy, -sy, sy, -sx * cy, sx * cy]
}

#[derive(Clone, Debug, Default)]
pub struct History {
    pub numbers: [Option<u64>; 6],
}

impl History {
    pub fn new(count: u64) -> Self {
        let count = count.max(1);
        let mut numbers = if count <= 6 {
            [1, 2, 3, 4, 5, 6].map(Some)
        } else {
            std::array::from_fn(|i| Some(count - i as u64))
        };
        // A responsive remount resets the pose, so keep the current count in front.
        if count <= 6 {
            numbers.swap(0, count as usize - 1);
        }
        Self { numbers }
    }

    pub fn record(&mut self, count: u64) -> usize {
        // The first six values are already printed; roll to them without repainting.
        if let Some(face) = self
            .numbers
            .iter()
            .position(|&number| number == Some(count))
        {
            return face;
        }
        let face = (0..6).min_by_key(|&i| self.numbers[i]).unwrap();
        self.numbers[face] = Some(count);
        face
    }
}

/// Retargetable three-axis winding with a fixed deadline after the latest click.
#[derive(Clone, Debug, Default)]
pub struct Roll {
    from: [f64; 3],
    to: [f64; 3],
    started_ms: f64,
}

impl Roll {
    pub fn sample(&self, now_ms: f64) -> ([f64; 3], [f64; 3]) {
        let remaining = (1.0 - (now_ms - self.started_ms) / SETTLE_MS).clamp(0.0, 1.0);
        let distance: [f64; 3] = std::array::from_fn(|i| self.to[i] - self.from[i]);
        (
            std::array::from_fn(|i| self.to[i] - distance[i] * remaining.powi(3)),
            distance.map(|d| 3.0 * d * remaining.powi(2) / SETTLE_MS),
        )
    }

    pub fn wind(&mut self, now_ms: f64, face: usize, turns: [i32; 3]) {
        let resting = self.finished(now_ms);
        if resting {
            // Remove full turns without changing the resting orientation.
            self.to = self.to.map(|angle| angle.rem_euclid(TURN));
            self.from = self.to;
        }
        let (angles, speeds) = self.sample(now_ms);
        for i in 0..3 {
            let direction = if resting {
                (turns[i] as f64).signum()
            } else {
                (self.to[i] - self.from[i]).signum()
            };
            // Keep momentum on interruption. Random extra turns vary the path,
            // retain the backlog, and increase speed even after a late click.
            let extra = TURN * turns[i].unsigned_abs().max(1) as f64;
            let distance = (self.to[i] - angles[i])
                .abs()
                .max(speeds[i].abs() * SETTLE_MS / 3.0)
                + extra;
            let target = REST_POSES[face][i];
            let turns_to_target = (angles[i] + direction * distance - target) / TURN;
            self.to[i] = target
                + TURN
                    * if direction > 0.0 {
                        turns_to_target.ceil()
                    } else {
                        turns_to_target.floor()
                    };
        }
        self.from = angles;
        self.started_ms = now_ms;
    }

    pub fn finished(&self, now_ms: f64) -> bool {
        self.to == self.from || now_ms >= self.started_ms + SETTLE_MS
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64) {
        assert!((a - b).abs() < 1e-7, "{a} != {b}");
    }

    #[test]
    fn history_uses_distinct_faces_then_replaces_only_the_oldest() {
        for initial_count in [1, 2, 5, 6, 100] {
            let mut history = History::new(initial_count);
            let mut previous = 0;
            for count in initial_count + 1..=initial_count + 100 {
                let before = history.numbers;
                let face = history.record(count);
                assert_ne!(face, previous);
                for i in 0..6 {
                    if i != face {
                        assert_eq!(before[i], history.numbers[i]);
                    }
                }
                let mut values: Vec<_> = history.numbers.into_iter().flatten().collect();
                values.sort();
                assert_eq!(
                    values,
                    (count.max(6) - 5..=count.max(6)).collect::<Vec<_>>()
                );
                previous = face;
            }
        }
        assert_eq!(
            History::new(100).numbers,
            [100, 99, 98, 97, 96, 95].map(Some)
        );
    }

    #[test]
    fn first_activation_spins_to_one_then_reuses_the_prefilled_faces() {
        let mut history = History::new(0);
        let initial = [1, 2, 3, 4, 5, 6].map(Some);
        assert_eq!(history.numbers, initial);
        let mut roll = Roll::default();
        let first = history.record(1);
        roll.wind(0.0, first, [1, -1, 1]);
        assert_ne!(roll.sample(SETTLE_MS / 2.0).0, [0.0; 3]);
        assert_eq!(
            roll.sample(SETTLE_MS).0.map(|a| a.rem_euclid(TURN)),
            REST_POSES[first]
        );
        assert_eq!(history.numbers[first], Some(1));
        for count in 2..=6 {
            let face = history.record(count);
            assert_eq!(history.numbers[face], Some(count));
            assert_eq!(history.numbers, initial);
        }
        assert_eq!(history.record(7), 0);
        assert_eq!(history.numbers, [7, 2, 3, 4, 5, 6].map(Some));
    }

    #[test]
    fn remounts_show_the_current_count_without_losing_prefilled_faces() {
        for count in [0, 1, 2, 3, 4, 5, 6, 7, 100, u64::MAX] {
            let history = History::new(count);
            assert_eq!(history.numbers[0], Some(count.max(1)));
            let mut values = history.numbers;
            values.sort();
            let newest = count.max(6);
            assert_eq!(values, std::array::from_fn(|i| Some(newest - 5 + i as u64)));
        }
    }

    #[test]
    fn every_face_lands_at_its_upright_pose_and_exact_deadline() {
        let mut roll = Roll::default();
        for face in 0..6 {
            let z = facing(REST_POSES[face]);
            assert_eq!(z.into_iter().filter(|&value| value > 0.001).count(), 1);
            close(z[face], 1.0);
            let now = face as f64 * 5000.0;
            let before = roll.sample(now).0;
            roll.wind(now, face, [-1, 2, -3]);
            for i in 0..3 {
                close((roll.sample(now).0[i] - before[i]).rem_euclid(TURN), 0.0);
                close(
                    (roll.sample(now + SETTLE_MS).0[i] - REST_POSES[face][i]).rem_euclid(TURN),
                    0.0,
                );
                close(roll.sample(now + SETTLE_MS).1[i], 0.0);
            }
            assert!(!roll.finished(now + SETTLE_MS - 1.0));
            assert!(roll.finished(now + SETTLE_MS));
        }
    }

    #[test]
    fn resting_faces_are_square_on_with_upright_text() {
        let normals = [
            [0., 0., 1.],
            [0., 0., -1.],
            [1., 0., 0.],
            [-1., 0., 0.],
            [0., -1., 0.],
            [0., 1., 0.],
        ];
        let up = [
            [0., -1., 0.],
            [0., -1., 0.],
            [0., -1., 0.],
            [0., -1., 0.],
            [0., 0., -1.],
            [0., 0., 1.],
        ];
        // Pax applies X, then Y, then Z rotation to each face's world basis.
        let rotate = |[x, y, z]: [f64; 3], pose: [f64; 3]| {
            let [rx, ry, rz] = pose.map(f64::to_radians);
            let (sx, cx) = rx.sin_cos();
            let (sy, cy) = ry.sin_cos();
            let (sz, cz) = rz.sin_cos();
            let (y, z) = (cx * y - sx * z, sx * y + cx * z);
            let (x, z) = (cy * x + sy * z, -sy * x + cy * z);
            [cz * x - sz * y, sz * x + cz * y, z]
        };
        for face in 0..6 {
            for (actual, expected) in rotate(normals[face], REST_POSES[face])
                .into_iter()
                .zip([0., 0., 1.])
            {
                close(actual, expected);
            }
            for (actual, expected) in rotate(up[face], REST_POSES[face])
                .into_iter()
                .zip([0., -1., 0.])
            {
                close(actual, expected);
            }
        }
    }

    #[test]
    fn interruptions_preserve_pose_backlog_and_boost_each_axis() {
        let mut roll = Roll::default();
        for (face, now) in [100.0, 100.0, 120.0, 180.0, 220.0, 500.0, 510.0, 1409.0]
            .into_iter()
            .enumerate()
        {
            let (angles, speeds) = roll.sample(now);
            let pending = std::array::from_fn::<_, 3, _>(|i| (roll.to[i] - angles[i]).abs());
            roll.wind(now, face % 6, [2, -1, 3]);
            for i in 0..3 {
                close(roll.sample(now).0[i], angles[i]);
                assert!(roll.sample(now).1[i].abs() > speeds[i].abs());
                assert!((roll.to[i] - angles[i]).abs() >= pending[i] + TURN);
            }
            assert!(!roll.finished(now + 999.0));
            assert!(roll.finished(now + 1000.0));
        }
    }

    #[test]
    fn random_paths_can_differ_but_settle_on_the_same_face() {
        let mut a = Roll::default();
        let mut b = Roll::default();
        a.wind(0.0, 3, [1, -2, 3]);
        b.wind(0.0, 3, [-3, 1, -1]);
        assert_ne!(a.sample(170.0).0, b.sample(170.0).0);
        for i in 0..3 {
            close(
                (a.sample(5000.0).0[i] - b.sample(5000.0).0[i]).rem_euclid(TURN),
                0.0,
            );
        }
    }
}
