use std::{cell::RefCell, collections::HashMap, hash::Hash};

use super::lowlink::Root;

/// Acts as a lint against incorrect usage of the various usize handles floating
/// around
///
/// This one is the primary handle used to manipulate nodes throughout the
/// algorithm
#[derive(Copy, Clone)]
pub(crate) struct Index(pub(crate) usize);

impl Index {
    pub(crate) fn into_root(self) -> Root {
        Root(self.0)
    }
}

/// Tracks nodes that we've already seen & assigns auto-incrementing indexes
///
/// The second property is important for the algorithm. Even though in practice
/// the nodes themselves are just newtyped integers, we require the property
/// that for any two nodes, the one we encounter earlier has a smaller index
/// than the one encountered later. See [`Lowlink`](super::lowlink::Lowlink)
pub(crate) struct IndexMap<Node>(RefCell<Inner<Node>>);

struct Inner<Node> {
    next_index: usize,
    forward: HashMap<Node, usize>,
    backward: HashMap<usize, Node>,
}

impl<Node: Copy + Hash + Eq> IndexMap<Node> {
    /// Constructor
    pub(super) fn new() -> Self {
        Self(RefCell::new(Inner {
            next_index: 0,
            forward: HashMap::new(),
            backward: HashMap::new(),
        }))
    }

    /// Check if the node is already in the map
    pub(crate) fn contains(&self, node: Node) -> bool {
        self.0.borrow().forward.contains_key(&node)
    }

    /// Forward lookup, given a node returns the [`Index`] which was assigned to
    /// it
    ///
    /// Panics if the node is not in the map
    #[track_caller]
    pub(crate) fn get(&self, node: Node) -> Index {
        assert!(self.contains(node), "Get called on unknown node");
        Index(self.0.borrow().forward[&node])
    }

    /// Backward lookup, given an [`Index`] returns the node which it was
    /// assigned to
    ///
    /// Panics if the node is not in the map
    #[track_caller]
    pub(crate) fn lookup(&self, Index(index): Index) -> Node {
        *self
            .0
            .borrow()
            .backward
            .get(&index)
            .expect("Lookup called on unknown node")
    }

    /// Insert a new node into the map
    ///
    /// Panics if called twice with the same node
    #[track_caller]
    pub(crate) fn insert(&self, node: Node) -> Index {
        assert!(!self.contains(node), "Cannot insert the same node twice");
        let mut this = self.0.borrow_mut();
        let index = this.next_index;
        this.next_index += 1;
        let _ = this.forward.insert(node, index);
        let _ = this.backward.insert(index, node);
        Index(index)
    }
}

#[cfg(test)]
mod tests {
    use super::{Index, IndexMap};

    #[test]
    fn add_series() {
        let map = IndexMap::new();
        assert!(matches!(map.insert(5), Index(0)));
        assert!(matches!(map.insert(17), Index(1)));
        assert!(matches!(map.insert(30), Index(2)));
    }

    #[test]
    #[should_panic(expected = "Cannot insert the same node twice")]
    fn double_add() {
        let map = IndexMap::new();
        let _ = map.insert(5);
        let _ = map.insert(5);
    }

    #[test]
    fn contains() {
        let map = IndexMap::new();
        let _ = map.insert(5);
        assert!(map.contains(5));
        assert!(!map.contains(4));
    }

    #[test]
    fn get() {
        let map = IndexMap::new();
        assert!(matches!(map.insert(5), Index(0)));
        assert!(matches!(map.get(5), Index(0)));
    }

    #[test]
    #[should_panic(expected = "Get called on unknown node")]
    fn get_panic() {
        let map = IndexMap::new();
        let _ = map.get(5);
    }

    #[test]
    fn lookup() {
        let map = IndexMap::new();
        assert!(matches!(map.insert(5), Index(0)));
        assert_eq!(map.lookup(Index(0)), 5);
    }

    #[test]
    #[should_panic(expected = "Lookup called on unknown node")]
    fn lookup_panic() {
        let map: IndexMap<i32> = IndexMap::new();
        let _ = map.lookup(Index(17));
    }
}
