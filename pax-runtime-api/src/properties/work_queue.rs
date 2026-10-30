//! FIFO reactive work indexed by graph. A preparing application must not make
//! another application's every effect/readiness check scan its pending work.
use super::private::PropertyId;
use std::collections::{BTreeMap, HashMap};

#[derive(Default)]
pub(super) struct WorkQueue {
    next: u64,
    owners: HashMap<Option<u64>, BTreeMap<u64, PropertyId>>,
    entries: HashMap<PropertyId, (Option<u64>, u64)>,
}
impl WorkQueue {
    pub fn push(&mut self, id: PropertyId, owner: Option<u64>) {
        if self.entries.contains_key(&id) {
            return;
        }
        let sequence = self.next;
        self.next = self
            .next
            .checked_add(1)
            .expect("reactive work sequence exhausted");
        self.entries.insert(id, (owner, sequence));
        self.owners.entry(owner).or_default().insert(sequence, id);
    }
    pub fn remove(&mut self, id: &PropertyId) {
        if let Some((owner, sequence)) = self.entries.remove(id) {
            let queue = self.owners.get_mut(&owner).unwrap();
            queue.remove(&sequence);
            if queue.is_empty() {
                self.owners.remove(&owner);
            }
        }
    }
    pub fn reassign(&mut self, id: PropertyId, owner: Option<u64>) {
        if let Some(&(previous, sequence)) = self.entries.get(&id) {
            if previous == owner {
                return;
            }
            self.remove(&id);
            self.entries.insert(id, (owner, sequence));
            self.owners.entry(owner).or_default().insert(sequence, id);
        }
    }
    pub fn len(&self, owner: Option<u64>) -> usize {
        owner.map_or(self.entries.len(), |owner| {
            self.owners.get(&Some(owner)).map_or(0, BTreeMap::len)
                + self.owners.get(&None).map_or(0, BTreeMap::len)
        })
    }
    pub fn pop(&mut self, owner: Option<u64>) -> Option<PropertyId> {
        let front = |owner| {
            self.owners
                .get(&owner)?
                .first_key_value()
                .map(|(&seq, &id)| (seq, id))
        };
        let candidate = if let Some(owner) = owner {
            [front(Some(owner)), front(None)]
                .into_iter()
                .flatten()
                .min_by_key(|(seq, _)| *seq)
        } else {
            self.owners
                .values()
                .filter_map(|queue| queue.first_key_value().map(|(&seq, &id)| (seq, id)))
                .min_by_key(|(seq, _)| *seq)
        };
        let (_, id) = candidate?;
        self.remove(&id);
        Some(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn owner_and_unowned_work_keep_fifo_without_draining_other_graphs() {
        let mut slots = slotmap::SlotMap::<PropertyId, ()>::with_key();
        let ids: Vec<_> = (0..6).map(|_| slots.insert(())).collect();
        let mut queue = WorkQueue::default();
        for (&id, owner) in ids
            .iter()
            .zip([Some(2), None, Some(1), Some(2), None, Some(1)])
        {
            queue.push(id, owner);
        }
        assert_eq!(queue.len(Some(1)), 4);
        assert_eq!(queue.pop(Some(1)), Some(ids[1]));
        assert_eq!(queue.pop(Some(1)), Some(ids[2]));
        queue.remove(&ids[4]);
        assert_eq!(queue.pop(Some(1)), Some(ids[5]));
        assert_eq!(queue.pop(Some(1)), None);
        assert_eq!(queue.pop(None), Some(ids[0]));
        assert_eq!(queue.pop(None), Some(ids[3]));
        assert_eq!(queue.len(None), 0);
        assert!(queue.owners.is_empty());
    }
    #[test]
    fn replacement_moves_pending_work_and_preserves_its_sequence() {
        let mut slots = slotmap::SlotMap::<PropertyId, ()>::with_key();
        let a = slots.insert(());
        let b = slots.insert(());
        let mut queue = WorkQueue::default();
        queue.push(a, Some(1));
        queue.push(b, Some(2));
        queue.push(a, Some(1));
        queue.reassign(a, Some(2));
        assert_eq!(queue.len(Some(1)), 0);
        assert_eq!(queue.pop(Some(2)), Some(a));
        assert_eq!(queue.pop(Some(2)), Some(b));
    }
}
