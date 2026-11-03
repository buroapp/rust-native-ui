//! Dense node-id → native-handle storage shared by every backend.
//!
//! Constitution IV prohibits allocation inside the layout and event-dispatch
//! path. Both native backends need constant-time lookup from a framework node id
//! to a platform handle on every change. A `HashMap` would allocate on growth and
//! pay hashing per access; a dense `Vec` indexed by the node id does neither.
//!
//! This is sound because the reconciler allocates node ids densely and
//! monotonically: id `n` is never issued before `n-1`, and ids are never reused.
//! Deletion leaves a hole rather than shifting, which keeps every other node's
//! index stable.
//!
//! Generic over the handle type so the Windows (`HWND`) and Linux (`gtk4::Widget`)
//! backends share one implementation, and so the logic is unit-testable on a host
//! that has neither toolkit.

use crate::NodeId;

/// A dense store mapping [`NodeId`] to an optional handle.
///
/// `None` means either "never created" or "deleted". The two are not
/// distinguished: callers only ever need to know whether a live handle exists.
#[derive(Debug)]
pub struct HandleStore<T> {
    slots: Vec<Option<T>>,
    live: usize,
}

impl<T> Default for HandleStore<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> HandleStore<T> {
    /// Create an empty store.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            slots: Vec::new(),
            live: 0,
        }
    }

    /// Create a store with room for `capacity` node ids.
    ///
    /// Pre-sizing is the documented way to keep `apply` allocation-free: call it
    /// once with the tree's node count before entering the render path.
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            slots: Vec::with_capacity(capacity),
            live: 0,
        }
    }

    /// Number of live handles.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.live
    }

    /// True when no handles are live.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.live == 0
    }

    /// Current allocation capacity in slots.
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.slots.capacity()
    }

    /// Insert or replace the handle for `node`.
    ///
    /// Returns the previous handle if one was present.
    pub fn insert(&mut self, node: NodeId, handle: T) -> Option<T> {
        let index = node.0 as usize;
        if index >= self.slots.len() {
            // Growth happens here and only here. Node ids arrive in increasing
            // order, so this is amortised O(1) and settles to zero once the tree
            // stops growing.
            self.slots.resize_with(index + 1, || None);
        }
        let previous = self.slots[index].replace(handle);
        if previous.is_none() {
            self.live += 1;
        }
        previous
    }

    /// Borrow the handle for `node`, if live.
    #[must_use]
    pub fn get(&self, node: NodeId) -> Option<&T> {
        self.slots.get(node.0 as usize).and_then(Option::as_ref)
    }

    /// Mutably borrow the handle for `node`, if live.
    pub fn get_mut(&mut self, node: NodeId) -> Option<&mut T> {
        self.slots
            .get_mut(node.0 as usize)
            .and_then(Option::as_mut)
    }

    /// Remove and return the handle for `node`.
    pub fn remove(&mut self, node: NodeId) -> Option<T> {
        let slot = self.slots.get_mut(node.0 as usize)?;
        let taken = slot.take();
        if taken.is_some() {
            self.live -= 1;
        }
        taken
    }

    /// True when a live handle exists for `node`.
    #[must_use]
    pub fn contains(&self, node: NodeId) -> bool {
        self.get(node).is_some()
    }

    /// Iterate live handles with their node ids.
    pub fn iter(&self) -> impl Iterator<Item = (NodeId, &T)> {
        self.slots
            .iter()
            .enumerate()
            .filter_map(|(i, slot)| slot.as_ref().map(|h| (NodeId(i as u64), h)))
    }

    /// Drop every handle, keeping the allocation.
    pub fn clear(&mut self) {
        for slot in &mut self.slots {
            *slot = None;
        }
        self.live = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_and_get_roundtrip() {
        let mut store: HandleStore<u32> = HandleStore::new();
        assert!(store.insert(NodeId(0), 100).is_none());
        assert!(store.insert(NodeId(3), 103).is_none());
        assert_eq!(store.get(NodeId(0)), Some(&100));
        assert_eq!(store.get(NodeId(3)), Some(&103));
        assert_eq!(store.len(), 2);
    }

    #[test]
    fn gaps_read_as_absent() {
        let mut store: HandleStore<u32> = HandleStore::new();
        store.insert(NodeId(5), 105);
        assert_eq!(store.get(NodeId(2)), None);
        assert!(!store.contains(NodeId(2)));
        assert!(store.contains(NodeId(5)));
    }

    #[test]
    fn remove_frees_slot_without_shifting() {
        let mut store: HandleStore<u32> = HandleStore::new();
        store.insert(NodeId(0), 1);
        store.insert(NodeId(1), 2);
        store.insert(NodeId(2), 3);

        assert_eq!(store.remove(NodeId(1)), Some(2));
        assert_eq!(store.len(), 2);
        // Neighbouring ids are unmoved, which is the whole point of a hole.
        assert_eq!(store.get(NodeId(0)), Some(&1));
        assert_eq!(store.get(NodeId(2)), Some(&3));
        assert_eq!(store.remove(NodeId(1)), None);
    }

    #[test]
    fn insert_over_existing_replaces_without_double_count() {
        let mut store: HandleStore<u32> = HandleStore::new();
        store.insert(NodeId(0), 1);
        let previous = store.insert(NodeId(0), 2);
        assert_eq!(previous, Some(1));
        assert_eq!(store.len(), 1);
        assert_eq!(store.get(NodeId(0)), Some(&2));
    }

    #[test]
    fn with_capacity_avoids_growth_during_fill() {
        let mut store: HandleStore<u32> = HandleStore::with_capacity(64);
        let cap_before = store.capacity();
        assert!(cap_before >= 64, "expected pre-sized capacity");

        for i in 0..64u64 {
            store.insert(NodeId(i), i as u32);
        }
        // No reallocation should have happened while filling the pre-sized range.
        assert_eq!(
            store.capacity(),
            cap_before,
            "store grew despite being pre-sized; render path would allocate"
        );
    }

    #[test]
    fn iter_yields_live_entries_in_id_order() {
        let mut store: HandleStore<u32> = HandleStore::new();
        store.insert(NodeId(0), 10);
        store.insert(NodeId(2), 12);
        store.remove(NodeId(2));
        store.insert(NodeId(4), 14);

        let entries: Vec<(u64, u32)> = store.iter().map(|(id, h)| (id.0, *h)).collect();
        assert_eq!(entries, vec![(0, 10), (4, 14)]);
    }

    #[test]
    fn clear_empties_without_dropping_capacity() {
        let mut store: HandleStore<u32> = HandleStore::with_capacity(8);
        store.insert(NodeId(0), 1);
        store.insert(NodeId(1), 2);
        store.clear();
        assert!(store.is_empty());
        assert_eq!(store.len(), 0);
        assert!(store.get(NodeId(0)).is_none());
    }

    #[test]
    fn get_mut_allows_in_place_edit() {
        let mut store: HandleStore<u32> = HandleStore::new();
        store.insert(NodeId(0), 1);
        if let Some(h) = store.get_mut(NodeId(0)) {
            *h = 99;
        }
        assert_eq!(store.get(NodeId(0)), Some(&99));
    }
}
