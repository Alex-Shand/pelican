use std::{collections::HashSet, hash::Hash};

use genawaiter::rc::Co;

use self::{
    index_map::{Index, IndexMap},
    lowlink::Lowlink,
    stack::Stack,
};
use super::Graph;

mod index_map;
mod lowlink;
mod stack;

pub(super) struct Tarjan<'a, Node: Copy + Hash + Eq> {
    co: &'a Co<HashSet<Node>>,
    graph: &'a Graph<Node>,
    index_map: IndexMap<Node>,
    stack: Stack,
    lowlink: Lowlink,
}

impl<'a, Node: Copy + Hash + Eq> Tarjan<'a, Node> {
    pub(super) fn new(
        co: &'a Co<HashSet<Node>>,
        graph: &'a Graph<Node>,
    ) -> Self {
        Self {
            co,
            graph,
            index_map: IndexMap::new(),
            stack: Stack::new(graph.size()),
            lowlink: Lowlink::new(graph.size()),
        }
    }
}

impl<Node: Copy + Hash + Eq> Tarjan<'_, Node> {
    /// Tarjan strongly connected component algorithm
    ///
    /// See [Lowlink] for an explanation of the algorithm
    pub(super) async fn tarjan(&self) {
        for node in self.graph.nodes() {
            if !self.index_map.contains(node) {
                let _ = self.tarjan_inner(node).await;
            }
        }
    }

    async fn tarjan_inner(&self, node: Node) -> Index {
        // This will only be called on a node which has no index, start by
        // giving it one. After this point everything handles the nodes using
        // the assigned index. This will panic if the node has already been
        // assigned an index
        let index = self.index_map.insert(node);
        // The stack tracks partial components
        self.stack.push(index);
        // This maps the each node to the root node of its strongly connected
        // component. We start by assuming each new node we encounter is in a
        // singleton component so we set the its root node to itself. This
        // panics if the node already has an assigned root
        self.lowlink.set(index, index.into_root());

        // Search through the node's children
        for child in self.graph.children(node).expect("Node should exist") {
            #[expect(clippy::if_not_else)]
            if !self.index_map.contains(child) {
                // If we've never seen this node before search through it too
                let child_index = Box::pin(self.tarjan_inner(child)).await;
                // The child might know of a better root (see the other branch)
                let child_root = self.lowlink.get(child_index);
                self.lowlink.update(index, child_root);
            } else {
                let child = self.index_map.get(child);
                if self.stack.contains(child) {
                    // If the child is already on the stack it is also an ancestor
                    // of this node which potentially makes it a better root node
                    // for this component
                    self.lowlink.update(index, child.into_root());
                }
            }
        }

        // If after all that this node is the root of its component then
        // everything higher on the stack is part of the component
        if self.lowlink.is_root(index) {
            let nodes = self
                .stack
                .pop_until(index)
                .map(|index| self.index_map.lookup(index))
                .collect();
            self.co.yield_(nodes).await;
        }

        index
    }
}

#[cfg(test)]
mod tests {
    use genawaiter::rc::Gen;

    use super::Tarjan;
    use crate::substitution::graph::Graph;

    fn make_graph() -> Graph<usize> {
        Graph::from_edges([
            // A square with corners 0, 1, 2, 3
            (0, 1),
            (1, 2),
            (2, 3),
            (3, 0),
            // A triangle with corners 4, 5, 6
            (4, 5),
            (5, 6),
            (6, 4),
            // A single directed edge connecting the two
            (4, 3),
        ])
    }

    macro_rules! set {
        ($($items: expr),* $(,)?) => {
            std::collections::HashSet::from([$($items),*])
        }
    }

    #[test]
    fn triangle() {
        let graph = make_graph();
        // The triangle is 'upstream' of the square so if we start from the
        // triangle we should find both
        let components = Gen::new(|co| async move {
            let _ = Tarjan::new(&co, &graph).tarjan_inner(4).await;
        })
        .into_iter()
        .collect::<Vec<_>>();
        assert_eq!(components, vec![set! {0, 1, 2, 3}, set! {4, 5, 6}]);
    }

    #[test]
    fn square() {
        let graph = make_graph();
        // Conversely if we start from the square we won't find the triangle
        let components = Gen::new(|co| async move {
            let _ = Tarjan::new(&co, &graph).tarjan_inner(0).await;
        })
        .into_iter()
        .collect::<Vec<_>>();
        assert_eq!(components, vec![set! {0, 1, 2, 3}]);
    }

    #[test]
    fn tarjan() {
        let graph = make_graph();
        // Thus we use a wrapper that calls the inner algorithm on every
        // unvisited node in order to make sure we get everything
        let components =
            Gen::new(
                |co| async move { Tarjan::new(&co, &graph).tarjan().await },
            )
            .into_iter()
            .collect::<Vec<_>>();
        assert_eq!(components, vec![set! {0, 1, 2, 3}, set! {4, 5, 6}]);
    }
}
