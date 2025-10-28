use std::cell::RefCell;

use genawaiter::rc::{Co, Gen};

use super::Index;

/// Used to track nodes which are part of a component we haven't fully
/// discovered yet
///
/// Spends a little extra memory to provide a constant time check for whether a
/// given node is currently somewhere on the stack
pub(crate) struct Stack(RefCell<Inner>);

struct Inner {
    stack: Vec<usize>,
    on_stack: Vec<bool>,
}

impl Stack {
    pub(crate) fn new(size: usize) -> Self {
        Self(RefCell::new(Inner {
            stack: Vec::with_capacity(size),
            on_stack: vec![false; size],
        }))
    }

    /// Check if a node is on the stack
    #[track_caller]
    pub(crate) fn contains(&self, Index(index): Index) -> bool {
        self.0.borrow().on_stack[index]
    }

    /// Push a node onto the stack
    #[track_caller]
    pub(crate) fn push(&self, Index(index): Index) {
        let mut this = self.0.borrow_mut();
        this.on_stack[index] = true;
        this.stack.push(index);
    }

    /// Pop nodes from the stack until the given node is reached. The argument
    /// node is popped as well
    ///
    /// Returns an iterator which must be consumed to actually remove nodes from
    /// the stack
    #[track_caller]
    pub(crate) fn pop_until(
        &self,
        index @ Index(node): Index,
    ) -> impl Iterator<Item = Index> {
        assert!(
            self.contains(index),
            "pop_until called with node not in the stack"
        );
        Gen::new(|co| async move {
            self.pop_until_inner(&co, node).await;
        })
        .into_iter()
    }

    async fn pop_until_inner(&self, co: &Co<Index>, node: usize) {
        loop {
            let popped = self.0.borrow_mut().pop();
            co.yield_(Index(popped)).await;
            if popped == node {
                return;
            }
        }
    }
}

impl Inner {
    #[track_caller]
    fn pop(&mut self) -> usize {
        let result = self.stack.pop().expect("Pop called on empty stack");
        self.on_stack[result] = false;
        result
    }
}

#[cfg(test)]
mod tests {
    use super::Stack;
    use crate::substitution::graph::tarjan::Index;

    #[test]
    fn push_and_contains() {
        let stack = Stack::new(5);
        stack.push(Index(4));
        stack.push(Index(2));
        assert!(stack.contains(Index(2)));
        assert!(stack.contains(Index(4)));
        assert!(!stack.contains(Index(0)));
        assert!(!stack.contains(Index(1)));
        assert!(!stack.contains(Index(3)));
    }

    #[test]
    #[should_panic(
        expected = "index out of bounds: the len is 0 but the index is 5"
    )]
    fn push_out_of_range() {
        let stack = Stack::new(0);
        stack.push(Index(5));
    }

    #[test]
    #[should_panic(
        expected = "index out of bounds: the len is 0 but the index is 5"
    )]
    fn contains_out_of_range() {
        let stack = Stack::new(0);
        let _ = stack.contains(Index(5));
    }

    #[test]
    fn pop() {
        let stack = Stack::new(5);
        stack.push(Index(4));
        stack.push(Index(3));
        assert!(stack.contains(Index(4)));
        assert!(stack.contains(Index(3)));

        assert_eq!(stack.0.borrow_mut().pop(), 3);
        assert!(!stack.contains(Index(3)));
        assert_eq!(stack.0.borrow_mut().pop(), 4);
        assert!(!stack.contains(Index(4)));
    }

    #[test]
    #[should_panic(expected = "Pop called on empty stack")]
    fn pop_empty() {
        let stack = Stack::new(5);
        let _ = stack.0.borrow_mut().pop();
    }

    #[test]
    fn pop_until() {
        let stack = Stack::new(5);
        stack.push(Index(4));
        stack.push(Index(2));
        stack.push(Index(0));

        let popped = stack.pop_until(Index(4)).map(|i| i.0).collect::<Vec<_>>();
        assert_eq!(popped, [0, 2, 4]);
    }

    #[test]
    #[should_panic(expected = "pop_until called with node not in the stack")]
    fn pop_until_invalid() {
        let stack = Stack::new(5);
        stack.push(Index(4));
        stack.push(Index(2));
        stack.push(Index(0));

        for _ in stack.pop_until(Index(3)) {}
    }
}
