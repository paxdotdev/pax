//! Queue one ticket per attached parent. Descendants share spatial priority across
//! renderer types, while FIFO age and the original mount identity survive a change of rank.
use super::PendingMount;
use crate::ExpandedNodeIdentifier;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::ops::Bound::{Excluded, Unbounded};

type Owner = (ExpandedNodeIdentifier, u64);
type Ticket = (u64, Owner);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Priority {
    Visible,
    Nearby,
    Background,
}

struct Group {
    priority: Priority,
    children: VecDeque<(u64, PendingMount)>,
}

#[derive(Default)]
pub(super) struct AdmissionQueue {
    groups: BTreeMap<Owner, Group>,
    by_priority: [BTreeSet<Ticket>; 3],
    by_age: BTreeSet<Ticket>,
    focus: Vec<Owner>,
    sequence: u64,
    turns: usize,
    refresh_after: Option<Owner>,
    pub(super) len: usize,
}

impl AdmissionQueue {
    pub(super) fn all_pending(&self) -> impl Iterator<Item = &PendingMount> {
        self.groups
            .values()
            .flat_map(|g| g.children.iter().map(|(_, work)| work))
    }

    pub(super) fn push(&mut self, work: PendingMount, priority: Priority) {
        let owner = (work.key.0, work.parent_epoch);
        let sequence = self.sequence;
        self.sequence += 1;
        self.len += 1;
        let group = self.groups.entry(owner).or_insert_with(|| Group {
            priority,
            children: VecDeque::new(),
        });
        if group.children.is_empty() {
            self.by_priority[priority as usize].insert((sequence, owner));
            self.by_age.insert((sequence, owner));
        }
        group.children.push_back((sequence, work));
    }

    pub(super) fn next_to_refresh(&mut self) -> Option<Owner> {
        let next = self
            .refresh_after
            .and_then(|owner| {
                self.groups
                    .range((Excluded(owner), Unbounded))
                    .next()
                    .map(|(&o, _)| o)
            })
            .or_else(|| self.groups.first_key_value().map(|(&o, _)| o));
        self.refresh_after = next;
        next
    }

    pub(super) fn rerank(&mut self, owner: Owner, priority: Priority) {
        let Some(group) = self.groups.get_mut(&owner) else {
            return;
        };
        if group.priority != priority {
            let ticket = (group.children[0].0, owner);
            self.by_priority[group.priority as usize].remove(&ticket);
            self.by_priority[priority as usize].insert(ticket);
            group.priority = priority;
        }
    }

    fn discovery_turn(&self) -> bool {
        self.turns % 8 == 7
    }

    pub(super) fn first(&self) -> Option<Owner> {
        // Within the visible tier, continue the current branch through layout,
        // templates and slots. A fairness turn still serves the oldest ticket.
        if !self.discovery_turn() && self.turns % 32 != 0 {
            if let Some(owner) = self.focus.iter().rev().find(|owner| {
                self.groups
                    .get(owner)
                    .is_some_and(|g| g.priority == Priority::Visible)
            }) {
                return Some(*owner);
            }
        }
        let ticket = if self.discovery_turn() {
            // Reserve one actual admission, including during an extended scroll.
            self.by_age.first()
        } else {
            self.by_priority.iter().find_map(BTreeSet::first)
        }?;
        Some(ticket.1)
    }

    pub(super) fn pop(&mut self) -> Option<PendingMount> {
        self.pop_owner(self.first()?)
    }

    pub(super) fn pop_specific(&mut self, request: &PendingMount) -> Option<PendingMount> {
        let owner = (request.key.0, request.parent_epoch);
        let g = self.groups.get_mut(&owner)?;
        let i = g.children.iter().position(|(_, v)| v.key == request.key)?;
        if i == 0 {
            return self.pop_owner(owner);
        }
        let (_, v) = g.children.remove(i)?;
        self.len -= 1;
        self.turns = self.turns.wrapping_add(1);
        Some(v)
    }

    fn pop_owner(&mut self, owner: Owner) -> Option<PendingMount> {
        let group = self.groups.get_mut(&owner).unwrap();
        let (sequence, work) = group.children.pop_front().unwrap();
        let lane = &mut self.by_priority[group.priority as usize];
        lane.remove(&(sequence, owner));
        self.by_age.remove(&(sequence, owner));
        if group.priority == Priority::Visible && self.turns % 8 != 7 {
            if let Some(index) = self.focus.iter().rposition(|id| *id == owner) {
                self.focus.truncate(index + 1);
            } else {
                self.focus.clear();
                self.focus.push(owner);
            }
            self.focus
                .push((work.key.2, work.child_epoch.wrapping_add(1)));
        }
        if let Some((sequence, _)) = group.children.front() {
            lane.insert((*sequence, owner));
            self.by_age.insert((*sequence, owner));
        } else {
            self.groups.remove(&owner);
        }
        self.turns = self.turns.wrapping_add(1);
        self.len -= 1;
        Some(work)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::rc::Weak;

    fn work(parent: u32, epoch: u64, child: u32) -> PendingMount {
        let key = (
            ExpandedNodeIdentifier(parent),
            epoch,
            ExpandedNodeIdentifier(child),
            0,
        );
        PendingMount {
            arrival: child,
            parent: Weak::new(),
            child: Weak::new(),
            parent_epoch: epoch,
            child_epoch: 0,
            key,
        }
    }

    #[test]
    fn viewport_reversal_preserves_fifo_age_and_the_exact_lifetime() {
        let mut q = AdmissionQueue::default();
        q.push(work(1, 1, 10), Priority::Background);
        q.push(work(1, 1, 11), Priority::Background);
        q.push(work(2, 1, 20), Priority::Visible);
        q.push(work(1, 2, 12), Priority::Nearby);
        assert_eq!(q.pop().unwrap().key.2 .0, 20);
        q.rerank((ExpandedNodeIdentifier(1), 1), Priority::Visible);
        assert_eq!(q.pop().unwrap().key.2 .0, 10);
        assert_eq!(q.pop().unwrap().key.2 .0, 11);
        let last = q.pop().unwrap();
        assert_eq!((last.key.2 .0, last.parent_epoch), (12, 2));
        assert!(q.pop().is_none());
    }

    #[test]
    fn continuous_visible_arrivals_cannot_starve_the_oldest_background_mount() {
        let mut q = AdmissionQueue::default();
        q.push(work(1, 1, 1), Priority::Background);
        for i in 2..=9 {
            q.push(work(i, 1, i), Priority::Visible);
            let got = q.pop().unwrap().key.2 .0;
            assert_eq!(got, if i == 9 { 1 } else { i });
        }
        assert_eq!(q.pop().unwrap().key.2 .0, 9);
        assert_eq!(q.len, 0);
    }

    #[test]
    fn visible_continuations_yield_to_reclassified_owners() {
        let mut q = AdmissionQueue::default();
        q.push(work(1, 1, 10), Priority::Visible);
        q.push(work(2, 1, 20), Priority::Visible);
        assert_eq!(q.pop().unwrap().key.2 .0, 10);
        q.push(work(10, 1, 11), Priority::Visible);
        assert_eq!(q.pop().unwrap().key.2 .0, 11);
        q.push(work(11, 1, 12), Priority::Visible);
        q.rerank((ExpandedNodeIdentifier(11), 1), Priority::Background);
        assert_eq!(q.pop().unwrap().key.2 .0, 20);
        assert_eq!(q.pop().unwrap().key.2 .0, 12);
    }

    #[test]
    fn refresh_cursor_visits_new_owners_after_removal_and_wraps() {
        let mut q = AdmissionQueue::default();
        for owner in [2, 4, 6] {
            q.push(work(owner, 1, owner), Priority::Nearby);
        }
        assert_eq!(q.next_to_refresh().unwrap().0 .0, 2);
        q.pop();
        q.push(work(1, 1, 1), Priority::Visible);
        let next: Vec<_> = (0..4).map(|_| q.next_to_refresh().unwrap().0 .0).collect();
        assert_eq!(next, [4, 6, 1, 4]);
    }
}
