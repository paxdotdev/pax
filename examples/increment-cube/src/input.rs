/// A poke belongs to the cube pressed, even if a different face rotates under
/// the pointer before release. Moving more than 12px cancels activation.
#[derive(Clone, Debug, Default)]
pub struct Poke {
    active: Option<(Option<i64>, f64, f64)>,
}

impl Poke {
    pub fn owns(&self, id: Option<i64>) -> bool {
        self.active.is_some_and(|(active, _, _)| active == id)
    }

    pub fn begin(&mut self, id: Option<i64>, x: f64, y: f64) -> bool {
        if self.active.is_some() {
            return false;
        }
        self.active = Some((id, x, y));
        true
    }

    pub fn moved(&mut self, id: Option<i64>, x: f64, y: f64) -> bool {
        if let Some((active, start_x, start_y)) = self.active {
            if active == id && (x - start_x).hypot(y - start_y) > 12.0 {
                return self.cancel(id);
            }
        }
        false
    }

    pub fn end(&mut self, id: Option<i64>, x: f64, y: f64) -> bool {
        self.moved(id, x, y);
        self.cancel(id)
    }

    pub fn cancel(&mut self, id: Option<i64>) -> bool {
        if self.owns(id) {
            self.active = None;
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn poke_requires_a_matching_sequence_and_activates_once() {
        let mut poke = Poke::default();
        assert!(!poke.end(None, 10.0, 20.0));
        assert!(poke.begin(Some(7), 10.0, 20.0));
        assert!(!poke.begin(Some(8), 10.0, 20.0));
        assert!(!poke.end(Some(8), 10.0, 20.0));
        assert!(poke.end(Some(7), 12.0, 22.0));
        assert!(!poke.end(Some(7), 12.0, 22.0));
    }

    #[test]
    fn drag_and_cancellation_never_activate_even_after_returning_to_origin() {
        let mut poke = Poke::default();
        poke.begin(None, 10.0, 20.0);
        assert!(poke.moved(None, 100.0, 20.0));
        assert!(!poke.end(None, 10.0, 20.0));
        poke.begin(None, 10.0, 20.0);
        assert!(!poke.end(None, 100.0, 20.0));
        poke.begin(None, 10.0, 20.0);
        assert!(poke.cancel(None));
        assert!(!poke.end(None, 10.0, 20.0));
    }
}
