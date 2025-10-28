//! Iterative substitution table

use std::collections::{HashMap, HashSet};

use value_type::value_type;

use self::graph::Graph;

mod graph;
#[cfg(test)]
mod tests;

/// Variable representing a table entry, used for recording [facts](Table::fact)
/// and adding [dependency](Table::dependency) relationships
#[value_type(Copy)]
pub struct Var(usize);

/// Value in the table
///
/// Provides a strategy for merging the values of two dependencies to contribute
/// to the ultimate value of this entry and a callback which is called if the
/// table finds a cyclic dependency
pub trait Value: Sized {
    #[allow(missing_docs)]
    type Error: std::error::Error;

    /// Called to merge the values of dependencies to produce a value for a row
    fn merge(left: Self, right: Self) -> Result<Self, Self::Error>;

    /// Called if a cyclic dependency is detected. The parameter is the partial
    /// result not counting the row itself
    fn resolve_cycle(known: Option<Self>) -> Result<Self, Self::Error>;
}

/// Returned by [`Table::fact`] if it is called twice with the same [`Var`]
#[value_type(Copy)]
#[derive(thiserror::Error)]
#[error("Duplicate entry for {0:?} in facts table")]
pub struct DuplicateFactError(pub Var);

/// Error returned by [`Table::resolve`]
#[derive(Debug, thiserror::Error)]
pub enum Error<E: std::error::Error> {
    /// Returned if the substitution process ceases to make progress
    #[error("Substitution stopped making progress")]
    NoProgress,
    /// Wraps [`Value::Error`]
    #[error(transparent)]
    Custom(#[from] E),
}

/// Iterative substitution table
#[expect(missing_debug_implementations)]
pub struct Table<T> {
    next_var: usize,
    known: HashMap<Var, T>,
    unknown: HashMap<Var, HashSet<Var>>,
}

impl<T> Default for Table<T> {
    fn default() -> Self {
        Self {
            next_var: 0,
            known: HashMap::new(),
            unknown: HashMap::new(),
        }
    }
}

impl<T: Clone> Table<T> {
    /// Constructor
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Produce a new [`Var`]
    pub fn var(&mut self) -> Var {
        let var = Var(self.next_var);
        self.next_var += 1;
        var
    }

    /// Record a known fact in the table
    ///
    /// Facts supercede dependencies e.g all of the following are equivalent
    /// ```
    /// # use pelican::substitution::Table;
    /// # #[derive(Copy, Clone)]
    /// # struct SomeValue;
    /// # impl pelican::substitution::Value for SomeValue {
    /// #     type Error = std::convert::Infallible;
    /// #     fn merge(_: Self, _: Self) -> Result<Self, Self::Error> {
    /// #         Ok(SomeValue)
    /// #     }
    /// #     fn resolve_cycle(_: Option<Self>) -> Result<Self, Self::Error> {
    /// #         Ok(SomeValue)
    /// #     }
    /// # }
    /// #
    /// # let mut table: pelican::substitution::Table<SomeValue> = Table::default();
    /// # let a = table.var();
    /// # let b = table.var();
    /// #
    /// let mut table = Table::default();
    /// table.fact(a, SomeValue).unwrap();
    /// table.dependency(a, b);
    ///
    /// let mut table = Table::default();
    /// table.dependency(a, b);
    /// table.fact(a, SomeValue).unwrap();
    ///
    /// let mut table = Table::default();
    /// table.fact(a, SomeValue).unwrap();
    /// ```
    pub fn fact(
        &mut self,
        var: Var,
        value: T,
    ) -> Result<(), DuplicateFactError> {
        if self.known.contains_key(&var) {
            return Err(DuplicateFactError(var));
        }
        let _ = self.known.insert(var, value);

        // Entries in known supercede entries in unknown
        let _ = self.unknown.remove(&var);

        Ok(())
    }

    /// Add a dependency to the table
    ///
    /// Facts supercede dependencies e.g all of the following are equivalent
    /// ```
    /// # use pelican::substitution::Table;
    /// # #[derive(Copy, Clone)]
    /// # struct SomeValue;
    /// # impl pelican::substitution::Value for SomeValue {
    /// #     type Error = std::convert::Infallible;
    /// #     fn merge(_: Self, _: Self) -> Result<Self, Self::Error> {
    /// #         Ok(SomeValue)
    /// #     }
    /// #     fn resolve_cycle(_: Option<Self>) -> Result<Self, Self::Error> {
    /// #         Ok(SomeValue)
    /// #     }
    /// # }
    /// #
    /// # let mut table: pelican::substitution::Table<SomeValue> = Table::default();
    /// # let a = table.var();
    /// # let b = table.var();
    /// #
    /// let mut table = Table::default();
    /// table.fact(a, SomeValue).unwrap();
    /// table.dependency(a, b);
    ///
    /// let mut table = Table::default();
    /// table.dependency(a, b);
    /// table.fact(a, SomeValue).unwrap();
    ///
    /// let mut table = Table::default();
    /// table.fact(a, SomeValue).unwrap();
    /// ```
    pub fn dependency(&mut self, var: Var, depends_on: Var) {
        // Entries in known supercede entries in unknown
        if self.known.contains_key(&var) {
            return;
        }
        let _ = self.unknown.entry(var).or_default().insert(depends_on);
    }

    /// Resolve the declared dependencies in the table
    pub fn resolve(self) -> Result<HashMap<Var, T>, Error<T::Error>>
    where
        T: Value,
    {
        // This is the table of resolved information, the goal is to move all of
        // the variables into this table. We start by populating it with our
        // initial set of facts
        let mut complete = self.known;
        // Partials holds the partial inference results
        let mut partials = Self::prepare_partials(self.unknown);
        // For unresolved partials in the loop below
        let mut next = HashMap::with_capacity(partials.len());

        // Loop until we run out of partials
        while !partials.is_empty() {
            let mut progress = false;

            // Check each currently unresolved variable
            for (var, partial) in partials {
                if complete.contains_key(&var) {
                    continue;
                }
                // Attempt to progress the partial result with respect to what
                // we know so far
                match partial.try_resolve(&complete)? {
                    TryResolveResult::Complete(result) => {
                        // If we resolved all of our dependencies record the
                        // result in the completed table and mark that we made
                        // progress
                        let _ = complete.insert(var, result);
                        progress = true;
                    }
                    TryResolveResult::Incomplete(partial, progressed) => {
                        // If we still have outstanding dependencies we store
                        // the new partial in the next table. In this case
                        // try_resolve also tells us if we managed to learn
                        // anything new this pass so record that too
                        let _ = next.insert(var, partial);
                        progress = progress || progressed;
                    }
                }
            }

            // If we made no progress, bail
            if !progress {
                return Err(Error::NoProgress);
            }

            // We've been putting anything unresolved in the next table, swap
            // that into the active one and drain the formerly active one
            partials = next;
            next = HashMap::with_capacity(partials.len());
        }

        Ok(complete)
    }

    // The major point of this and the reason we can't just use the original
    // unknown table directly for resolution has to do with cycles in the
    // dependency graph.
    //
    // We start by finding all of the strongly connected components in the
    // dependency graph, this is a slightly wider condition than true cycles
    //
    // Consider a single strongly connected component with no incomming or
    // outgoing edges. By the definition of a strongly connected component every
    // node in the component is reachable by some path from every other node.
    // Since our edges are dependencies that means every node in the component
    // is (transitivley) dependent on every other node in the component and
    // ultimatly on itself. As there are no other outgoing nodes there is no
    // extra information to be had and every node in the component must resolve
    // to the same value in the event we can decide what value that should be.
    // So we can replace the entire component with a single 'virtual' node with
    // one dependency edge pointing back at itself, this is a situation we know
    // how to resolve.
    //
    // Now consider a component with some number of outgoing edges (e.g some
    // nodes inside the component depend on some nodes outside of the
    // component). Because any node inside the component reachable from any
    // other node then there is a path from every node inside the component to
    // every non-component node depended on by any node in the component. So, to
    // labor the point, every node in the component (transitivley) depends on
    // every non-component node depended on by any component node. Similar to
    // the first case we can achive the same affect by collapsing the entire
    // component into a single virtual node with a recursive dependency & a
    // dependency on each of the non-component nodes dependended on by any node
    // in the component.
    //
    // Finally consider incoming dependency edges. After collapsing the
    // component into a single node we could go through the entire graph and
    // patch up any dependency edge targeting any component node to refer
    // instead to the virtual node. This is awkward though since we index edges
    // by source node so we have to traverse every other edge in the graph in
    // order to find edges which travel into the component. Since the premise of
    // the virtual node transformation is that every node in the component is
    // essentially identical we can instead make very component node look like
    // the proposed virtual node (e.g with a direct dependency for each edge
    // leaving the component and one recursive dependency edge). This has the
    // same affect as the virtual node approach but means we don't need to patch
    // up incoming edges or translate the virtual node(s) back to the original
    // nodes after inference
    fn prepare_partials(
        unknown: HashMap<Var, HashSet<Var>>,
    ) -> HashMap<Var, Partial<T>> {
        let mut graph = Graph::new();
        for (src, dsts) in unknown {
            graph.add_edges(src, &dsts);
        }

        // Compute all of the strongly connected components of the graph
        let sccs = graph.strongly_connected_components().collect::<Vec<_>>();

        // For each of them
        for component in sccs {
            // Compute the set of dependencies of the component, this is the
            // union of all of the dependencies of all of the nodes in the
            // component minus any nodes which are themselves members of the
            // component
            let all_dependencies = component
                .iter()
                .filter_map(|&node| graph.children(node))
                .flatten()
                .filter(|node| !component.contains(node))
                .collect();
            // For each node in the component we delete all of the original
            // edges it had and add one for each of the components dependencies
            // and one recursive edge
            for node in component {
                graph.delete_outgoing_edges(node);
                graph.add_edges(node, &all_dependencies);
                graph.add_edge(node, node);
            }
        }

        // Now we can build our partials table
        let mut result = HashMap::new();
        for (var, mut dependencies) in graph {
            let recursive = dependencies.remove(&var);
            let _ = result.insert(
                var,
                Partial {
                    recursive,
                    result: None,
                    dependencies,
                },
            );
        }

        result
    }
}

/// Partial result during inference
struct Partial<T> {
    // True if the variable assigned to this partial depends on itself
    recursive: bool,
    // Partial result, if known
    result: Option<T>,
    // Remaining dependencies, if any
    dependencies: HashSet<Var>,
}

enum TryResolveResult<T> {
    Complete(T),
    Incomplete(Partial<T>, bool),
}

impl<T: Clone> Partial<T> {
    fn try_resolve(
        self,
        known: &HashMap<Var, T>,
    ) -> Result<TryResolveResult<T>, Error<T::Error>>
    where
        T: Value,
    {
        let Self {
            recursive,
            result,
            dependencies,
        } = self;
        let mut new_result = None;
        let mut new_dependencies = HashSet::new();
        for dep in dependencies {
            // If we have a value for the variable we merge it into the result,
            // otherwise it goes back in the dependency set
            if let Some(known) = known.get(&dep) {
                new_result = merge_opt(new_result, Some(known.clone()))?;
            } else {
                let _ = new_dependencies.insert(dep);
            }
        }

        // If new_result contains something then we learned something new from
        // this pass
        let progressed = new_result.is_some();
        let result = merge_opt(result, new_result)?;

        // If we still have dependencies to resolve the result is always
        // Incomplete
        if !new_dependencies.is_empty() {
            return Ok(TryResolveResult::Incomplete(
                Self {
                    recursive,
                    result,
                    dependencies: new_dependencies,
                },
                progressed,
            ));
        }

        // If our last remaining dependency is a recursive edge we can ask the
        // type what the answer should be
        if recursive {
            return Ok(TryResolveResult::Complete(T::resolve_cycle(result)?));
        }

        // Finally if we're not recursive and we don't have a partial result
        // then we're stuck
        let Some(result) = result else {
            return Err(Error::NoProgress);
        };

        Ok(TryResolveResult::Complete(result))
    }
}

fn merge_opt<T: Value>(
    left: Option<T>,
    right: Option<T>,
) -> Result<Option<T>, T::Error> {
    match (left, right) {
        (None, None) => Ok(None),
        (Some(left), None) => Ok(Some(left)),
        (None, Some(right)) => Ok(Some(right)),
        (Some(left), Some(right)) => Ok(Some(T::merge(left, right)?)),
    }
}
