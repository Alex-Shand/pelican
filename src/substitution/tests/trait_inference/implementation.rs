use std::{collections::HashMap, convert::Infallible};

use crate::substitution::{Error, Table, Value, Var};

/// Simplified version of trait inference, a tree structure where leaf nodes
/// either have The Property (TM) or don't. Internal nodes have a list of
/// children, they have The Property (TM) either if they have no children or if
/// all of their children have The Property (TM)
#[derive(Debug, Clone)]
pub(super) struct Ast(pub(super) HashMap<usize, Node>);

#[derive(Debug, Clone)]
pub(super) enum Node {
    Leaf(bool),
    Internal(Vec<usize>),
}

/// Output, tracks for both internal and external nodes whether they have The
/// Property (TM)
#[derive(Debug, PartialEq)]
pub(super) struct TypedAst(pub(super) HashMap<usize, TypedNode>);
#[derive(Debug, PartialEq)]
pub(super) enum TypedNode {
    Leaf(bool),
    Internal(Vec<usize>, bool),
}

impl Value for bool {
    type Error = Infallible;

    // A given item only has The Property (TM) if all of it's members have The
    // Property (TM)
    fn merge(left: Self, right: Self) -> Result<Self, Self::Error> {
        Ok(left && right)
    }

    // In the event of a cyclic dependency we go with the result from the other
    // dependencies if present, and default to true if this is the only
    // dependency
    fn resolve_cycle(known: Option<Self>) -> Result<Self, Self::Error> {
        Ok(known.unwrap_or(true))
    }
}

struct Engine {
    table: Table<bool>,
    id_to_var: HashMap<usize, Var>,
}

impl Engine {
    fn new() -> Self {
        Self {
            table: Table::new(),
            id_to_var: HashMap::new(),
        }
    }

    fn resolve(
        mut self,
        Ast(ast): &Ast,
    ) -> Result<HashMap<usize, bool>, Error<Infallible>> {
        // Populate dependencies
        for (id, node) in ast {
            let var = self.get_var(*id);
            match node {
                Node::Leaf(p) => {
                    self.table.fact(var, *p).expect("Duplicate key in hashmap");
                }
                Node::Internal(dependencies) => {
                    if dependencies.is_empty() {
                        self.table
                            .fact(var, true)
                            .expect("Duplicate key in hashmap");
                    } else {
                        for dep in dependencies {
                            let dep = self.get_var(*dep);
                            self.table.dependency(var, dep);
                        }
                    }
                }
            }
        }
        dbg!(&self.table.known);
        dbg!(&self.table.unknown);

        // Resolve
        let result = self.table.resolve()?;
        dbg!(&result);

        // Substitute the original ids
        let mut var_to_id = self
            .id_to_var
            .into_iter()
            .map(|(a, b)| (b, a))
            .collect::<HashMap<_, _>>();
        Ok(result
            .into_iter()
            .map(|(var, result)| {
                (
                    var_to_id.remove(&var).expect("Duplicate key in hashmap"),
                    result,
                )
            })
            .collect())
    }

    fn get_var(&mut self, id: usize) -> Var {
        let &mut var =
            self.id_to_var.entry(id).or_insert_with(|| self.table.var());
        var
    }
}

pub(super) fn infer(ast: Ast) -> Result<TypedAst, Error<Infallible>> {
    let resolved = Engine::new().resolve(&ast)?;

    let mut result = HashMap::new();
    for (id, node) in ast.0 {
        dbg!(id);
        let node = match node {
            Node::Leaf(expected) => {
                let actual = resolved[&id];
                debug_assert_eq!(expected, actual);
                TypedNode::Leaf(expected)
            }
            Node::Internal(items) => TypedNode::Internal(items, resolved[&id]),
        };
        let old = result.insert(id, node);
        debug_assert!(old.is_none());
    }

    Ok(TypedAst(result))
}
