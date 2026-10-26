// Generation-tagged IDs are local to each module; no Rust pointer crosses WASM.
// Retire a slot instead of allowing its generation to wrap and revive stale IDs.
pub struct Arena<T> {
    slots: Vec<(u16, Option<T>)>,
    free: Vec<usize>,
}
impl<T> Default for Arena<T> {
    fn default() -> Self {
        Self {
            slots: vec![],
            free: vec![],
        }
    }
}
impl<T> Arena<T> {
    pub fn insert(&mut self, value: T) -> u32 {
        let i = self.free.pop().unwrap_or_else(|| {
            assert!(self.slots.len() < 65535);
            self.slots.push((1, None));
            self.slots.len() - 1
        });
        self.slots[i].1 = Some(value);
        ((self.slots[i].0 as u32) << 16) | (i as u32 + 1)
    }
    pub fn get(&self, id: u32) -> &T {
        self.try_get(id).expect("stale module handle")
    }
    pub fn try_get(&self, id: u32) -> Option<&T> {
        let (generation, value) = self.slots.get((id as u16 as usize).checked_sub(1)?)?;
        (*generation as u32 == id >> 16)
            .then_some(value.as_ref())
            .flatten()
    }
    pub fn get_mut(&mut self, id: u32) -> &mut T {
        let (generation, value) = &mut self.slots[(id as u16 as usize) - 1];
        assert_eq!(*generation as u32, id >> 16);
        value.as_mut().expect("released handle")
    }
    pub fn remove(&mut self, id: u32) -> T {
        let i = (id as u16 as usize) - 1;
        let (generation, value) = &mut self.slots[i];
        assert_eq!(*generation as u32, id >> 16);
        let value = value.take().expect("released handle");
        if *generation < u16::MAX {
            *generation += 1;
            self.free.push(i);
        }
        value
    }
    pub fn len(&self) -> usize {
        self.slots.iter().filter(|(_, v)| v.is_some()).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn released_handles_never_alias_reused_slots() {
        let mut arena = Arena::default();
        let first = arena.insert(10);
        assert_eq!(arena.remove(first), 10);
        let second = arena.insert(20);
        assert_ne!(first, second);
        assert!(arena.try_get(first).is_none());
        assert_eq!(*arena.get(second), 20);
    }
    #[test]
    fn exhausted_generation_retires_the_slot() {
        let mut arena = Arena::default();
        let first = arena.insert(());
        let mut handle = first;
        for _ in 0..u16::MAX {
            arena.remove(handle);
            handle = arena.insert(());
            assert!(arena.try_get(first).is_none());
        }
        assert_eq!(handle as u16, 2);
        assert_eq!(arena.len(), 1);
    }
}
