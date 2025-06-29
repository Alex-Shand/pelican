use std::collections::HashMap;

use implementation::{TypedAst, TypedNode};

use self::implementation::{Ast, Node, infer};

mod implementation;

type Result<T, E = Box<dyn std::error::Error>> = std::result::Result<T, E>;

macro_rules! map {
    ($($key:literal : $value:expr),* $(,)?) => {
        HashMap::from([$(($key, $value)),*])
    }
}

#[test]
fn empty() -> Result<()> {
    let ast = Ast(map! {});
    let expected = TypedAst(map! {});
    let result = infer(ast)?;
    assert_eq!(result, expected);
    Ok(())
}

#[test]
fn leaf_only() -> Result<()> {
    let ast = Ast(map! {
        0: Node::Leaf(true),
        1: Node::Leaf(false),
    });
    let expected = TypedAst(map! {
        0: TypedNode::Leaf(true),
        1: TypedNode::Leaf(false),
    });
    let result = infer(ast)?;
    assert_eq!(result, expected);
    Ok(())
}

#[test]
fn internal_only() -> Result<()> {
    let ast = Ast(map! {
        0: Node::Internal(vec![]),
        1: Node::Internal(vec![0]),
        2: Node::Internal(vec![0, 1]),
    });
    let expected = TypedAst(map! {
        0: TypedNode::Internal(vec![], true),
        1: TypedNode::Internal(vec![0], true),
        2: TypedNode::Internal(vec![0, 1], true),
    });
    let result = infer(ast)?;
    assert_eq!(result, expected);
    Ok(())
}

#[test]
fn tree() -> Result<()> {
    /*
            0
         ┌──┴──┐
         1     2
       ┌─┴──┬──┴─┐
       3    4    5
    3: true
    4: true
    5: false
    1: true (as both children are true)
    2: false (as 5 is false)
    0: false (as 2 is false)
    */
    let ast = Ast(map! {
        0: Node::Internal(vec![1, 2]),
        1: Node::Internal(vec![3, 4]),
        2: Node::Internal(vec![4, 5]),
        3: Node::Leaf(true),
        4: Node::Leaf(true),
        5: Node::Leaf(false),
    });
    let expected = TypedAst(map! {
        0: TypedNode::Internal(vec![1, 2], false),
        1: TypedNode::Internal(vec![3, 4], true),
        2: TypedNode::Internal(vec![4, 5], false),
        3: TypedNode::Leaf(true),
        4: TypedNode::Leaf(true),
        5: TypedNode::Leaf(false),
    });
    let result = infer(ast)?;
    assert_eq!(result, expected);
    Ok(())
}

#[test]
fn cycle() -> Result<()> {
    let ast = Ast(map! {
        0: Node::Internal(vec![5]),
        1: Node::Internal(vec![0]),
        2: Node::Internal(vec![1]),
        3: Node::Internal(vec![2]),
        4: Node::Internal(vec![3]),
        5: Node::Internal(vec![4]),
    });
    let expected = TypedAst(map! {
        0: TypedNode::Internal(vec![5], true),
        1: TypedNode::Internal(vec![0], true),
        2: TypedNode::Internal(vec![1], true),
        3: TypedNode::Internal(vec![2], true),
        4: TypedNode::Internal(vec![3], true),
        5: TypedNode::Internal(vec![4], true),
    });
    let result = infer(ast)?;
    assert_eq!(result, expected);
    Ok(())
}

#[test]
fn messy_cycle() -> Result<()> {
    let ast = Ast(map! {
        0: Node::Internal(vec![2, 3]),
        1: Node::Internal(vec![0, 4]),
        2: Node::Internal(vec![1, 5]),
        3: Node::Leaf(true),
        4: Node::Leaf(false),
        5: Node::Leaf(true),
    });
    let expected = TypedAst(map! {
        0: TypedNode::Internal(vec![2, 3], false),
        1: TypedNode::Internal(vec![0, 4], false),
        2: TypedNode::Internal(vec![1, 5], false),
        3: TypedNode::Leaf(true),
        4: TypedNode::Leaf(false),
        5: TypedNode::Leaf(true),
    });
    let result = infer(ast)?;
    assert_eq!(result, expected);
    Ok(())
}

#[test]
fn double_cycle() -> Result<()> {
    let ast = Ast(map! {
        0: Node::Internal(vec![2, 4]),
        1: Node::Internal(vec![0, 5]),
        2: Node::Internal(vec![1, 6]),
        3: Node::Internal(vec![0, 7]),
        4: Node::Internal(vec![3, 8]),
        5: Node::Leaf(true),
        6: Node::Leaf(false),
        7: Node::Leaf(true),
        8: Node::Leaf(false),
    });
    let expected = TypedAst(map! {
        0: TypedNode::Internal(vec![2, 4], false),
        1: TypedNode::Internal(vec![0, 5], false),
        2: TypedNode::Internal(vec![1, 6], false),
        3: TypedNode::Internal(vec![0, 7], false),
        4: TypedNode::Internal(vec![3, 8], false),
        5: TypedNode::Leaf(true),
        6: TypedNode::Leaf(false),
        7: TypedNode::Leaf(true),
        8: TypedNode::Leaf(false),
    });
    let result = infer(ast)?;
    assert_eq!(result, expected);
    Ok(())
}
