use std::cell::RefCell;

use super::Index;

/// To help distinguish the two arguments to [`set`](Lowlink::set) &
/// [`update`](Lowlink::update)
#[derive(Copy, Clone)]
pub(super) struct Root(pub(super) usize);

/// The goal of this object is to track for each node, the node with the lowest
/// index which is a member of the same Strongly Connected Component (scc).
/// Keeping track of this information is how we keep track of whether we've seen
/// an entire scc.
///
/// The process works as follows:
/// * Whenever we encounter a node we've never seen before we ask
///   [`IndexMap`](super::index_map::IndexMap) to give it a new index. Because
///   IndexMap assigns incrementing indexes this index will be higher than any
///   other we've seen so far
/// * The node starts by assigning its Lowlink value to itself & push it onto
///   the stack
/// * For each of the node's childen
///     * If we've seen it before and it isn't on the stack then it is part of
///       an scc which we've already fully explored and returned, we ignore it
///     * If we've seen it before and it is on the stack then it must be part of
///       the current scc we're exploring, because it is both reachable from
///       this node and an ancestor of this node. We may choose to update our
///       own low link to this nodes *index* if it's better (lower) than what we
///       have currently
///     * If we've never seen it before it may be part of this scc or we may
///       have an edge that leaves the current scc and enters a new one. We
///       start by traversing through the new node
///         * At the end of this process the new node will have some value set
///           for it's own lowlink
///         * If this value is less than or equal to ours then the node was part
///           of this scc (because the true root node with the lowest index must
///           be reachable via some path from the child). We can update our own
///           lowlink value to the childs in the event it's better
///         * If the child's lowlink is greater than ours then it is part of a
///           different scc which up until this point we hadn't explored. By the
///           time the recursive call terminates we must have explored the the
///           entirety of the new scc so we can ignore the child node
///
/// The goal of all of this is to detect the first node from the scc (henceforth
/// refered to as the root node of the scc) on the way back up through the
/// recurisve calls. At this point we know the following things about the nodes
/// involved in the scc
///  * The root node is the first node from the scc we encountered (by
///    construction) so it will have the lowest index of all the nodes in the
///    scc
/// * The root node is the only node in the scc where every other node in the
///   scc is a strict child (e.g we reach it via the DFS step)
/// * For every other node in the scc, there is a path back to the root node. As
///   a result one of the non-root node's decendents must also be it's ancestor.
/// * This means that for every non-root node the lowlink value will be lower
///   than that node's index
/// * So the once we've hit every node in the scc, the only node where the
///   lowlink is still equal to the node's index is the root node
///
/// We need to know the root node because as we've been traversing the scc we've
/// been pushing the nodes we find onto the stack. We also could have any number
/// of incomplete sccs on the stack caused by taking an edge out of one scc into
/// another during the DFS step. As the root node is the first node from the scc
/// we saw it will be lowest on the stack. As we will have completed any new
/// sccs we wandered into during traversal of this scc, all of the nodes higher
/// than the root node on the stack are part of this scc.
///
/// So once the recursion unwinds to the frame where we detect the root node we
/// can pop everything off of the stack up to the root node and that is our scc
pub(super) struct Lowlink(RefCell<Vec<usize>>);

impl Lowlink {
    /// Constructor
    pub(super) fn new(size: usize) -> Self {
        // The update operation is min(current, new) so we use usize::MAX as the
        // sentinel value for an unoccupied slot. This should be fine because
        // practically speaking (on a 64 bit OS) we will never have enough
        // memory for usize::MAX nodes
        Self(RefCell::new(vec![usize::MAX; size]))
    }

    /// Look up the current lowlink root for a given node index
    ///
    /// Panics if the node has no assigned lowlink
    #[track_caller]
    pub(super) fn get(&self, Index(node): Index) -> Root {
        let root = self.0.borrow()[node];
        assert_ne!(root, usize::MAX, "node has no lowlink assigned");
        Root(root)
    }

    /// Check if a node is its own lowlink root
    #[track_caller]
    pub(super) fn is_root(&self, Index(node): Index) -> bool {
        self.0.borrow()[node] == node
    }

    /// Set the lowlink root for a node
    ///
    /// Panics if the node already has an assigned root
    #[track_caller]
    pub(super) fn set(&self, Index(node): Index, Root(root): Root) {
        assert_eq!(self.0.borrow()[node], usize::MAX, "lowlink is already set");
        self.0.borrow_mut()[node] = root;
    }

    /// Update a node's lowlink root if the new one is better (lower) than the
    /// current one
    #[track_caller]
    pub(super) fn update(&self, Index(node): Index, Root(new): Root) {
        let current = self.0.borrow()[node];
        self.0.borrow_mut()[node] = usize::min(current, new);
    }
}

#[cfg(test)]
mod tests {
    use super::{Lowlink, Root};
    use crate::substitution::graph::tarjan::Index;

    #[test]
    fn happy() {
        let lowlink = Lowlink::new(5);
        lowlink.set(Index(0), Root(5));
        assert!(matches!(lowlink.get(Index(0)), Root(5)));
        lowlink.update(Index(0), Root(1));
        assert!(matches!(lowlink.get(Index(0)), Root(1)));
        lowlink.update(Index(0), Root(3));
        assert!(matches!(lowlink.get(Index(0)), Root(1)));
        lowlink.update(Index(0), Root(0));
        assert!(lowlink.is_root(Index(0)));
    }

    #[test]
    #[should_panic(expected = "node has no lowlink assigned")]
    fn get_unset() {
        let lowlink = Lowlink::new(5);
        let _ = lowlink.get(Index(0));
    }

    #[test]
    #[should_panic(
        expected = "index out of bounds: the len is 0 but the index is 4"
    )]
    fn get_out_of_range() {
        let lowlink = Lowlink::new(0);
        let _ = lowlink.get(Index(4));
    }

    #[test]
    #[should_panic(expected = "lowlink is already set")]
    fn double_set() {
        let lowlink = Lowlink::new(5);
        lowlink.set(Index(0), Root(5));
        assert!(matches!(lowlink.get(Index(0)), Root(5)));
        lowlink.set(Index(0), Root(4));
    }

    #[test]
    #[should_panic(
        expected = "index out of bounds: the len is 0 but the index is 4"
    )]
    fn set_out_of_range() {
        let lowlink = Lowlink::new(0);
        lowlink.set(Index(4), Root(5));
    }

    #[test]
    #[should_panic(
        expected = "index out of bounds: the len is 0 but the index is 4"
    )]
    fn update_out_of_range() {
        let lowlink = Lowlink::new(0);
        lowlink.update(Index(4), Root(5));
    }

    #[test]
    #[should_panic(
        expected = "index out of bounds: the len is 0 but the index is 4"
    )]
    fn is_root_out_of_range() {
        let lowlink = Lowlink::new(0);
        let _ = lowlink.is_root(Index(4));
    }
}
