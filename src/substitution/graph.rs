//! A simple directed graph

use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
};

use genawaiter::rc::Gen;
use tarjan::Tarjan;

mod tarjan;

#[derive(Debug)]
pub(crate) struct Graph<Node: Copy + Hash + Eq>(HashMap<Node, HashSet<Node>>);

impl<Node: Copy + Hash + Eq> Default for Graph<Node> {
    fn default() -> Self {
        Self(HashMap::default())
    }
}

#[cfg(test)]
impl Graph<usize> {
    fn from_edges(edges: impl IntoIterator<Item = (usize, usize)>) -> Self {
        let mut this = Self::new();
        for (start, end) in edges {
            this.add_edge(start, end);
        }
        this
    }
}

impl<Node: Copy + Hash + Eq> Graph<Node> {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn add_edge(&mut self, start: Node, end: Node) {
        let _ = self.0.entry(start).or_default().insert(end);
        let _ = self.0.entry(end).or_default();
    }

    pub(crate) fn add_edges(&mut self, start: Node, ends: &HashSet<Node>) {
        for end in ends {
            self.add_edge(start, *end);
        }
    }

    pub(crate) fn delete_outgoing_edges(&mut self, node: Node) {
        let _ = self.0.insert(node, HashSet::new());
    }

    pub(crate) fn size(&self) -> usize {
        self.0.len()
    }

    pub(crate) fn nodes(&self) -> impl Iterator<Item = Node> {
        self.0.keys().copied()
    }

    pub(crate) fn children(
        &self,
        node: Node,
    ) -> Option<impl Iterator<Item = Node>> {
        let children = self.0.get(&node)?;
        Some(children.iter().copied())
    }

    pub(crate) fn strongly_connected_components(
        &self,
    ) -> impl Iterator<Item = HashSet<Node>> {
        Gen::new(|co| async move { Tarjan::new(&co, self).tarjan().await })
            .into_iter()
    }
}

impl<Node: Copy + Hash + Eq> IntoIterator for Graph<Node> {
    type Item = (Node, HashSet<Node>);

    type IntoIter = <HashMap<Node, HashSet<Node>> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::Graph;

    macro_rules! set {
        ($($items: expr),* $(,)?) => {
            std::collections::HashSet::from([$($items),*])
        }
    }

    #[test]
    fn nodes() {
        let graph = Graph::from_edges([(0, 1), (0, 2), (0, 3)]);
        let nodes = graph.nodes().collect::<HashSet<_>>();
        assert_eq!(nodes, set! {0, 1, 2, 3});
    }

    #[test]
    fn children() {
        let graph = Graph::from_edges([(0, 1), (0, 2), (0, 3)]);
        assert_eq!(
            graph.children(0).map(Iterator::collect),
            Some(set! {1, 2, 3})
        );
        assert_eq!(graph.children(1).map(Iterator::collect), Some(set! {}));
        assert_eq!(graph.children(2).map(Iterator::collect), Some(set! {}));
        assert_eq!(graph.children(3).map(Iterator::collect), Some(set! {}));
        assert!(graph.children(4).is_none());
    }

    #[test]
    fn strongly_connected_components() {
        let graph = Graph::from_edges([
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
        ]);
        let components =
            graph.strongly_connected_components().collect::<Vec<_>>();
        assert_eq!(components, vec![set! {0, 1, 2, 3}, set! {4, 5, 6}]);
    }
}
