use pretty_assertions::assert_eq;
use trivial::Trivial as _;

use self::{
    builders::*,
    implementation::{TypeError, infer},
};
use crate::unification::Var;

mod builders;
mod implementation;

macro_rules! set {
    ($($tt:tt)*) => {
        vec![$($tt)*].into_iter().collect::<::std::collections::HashSet<_>>()
    };
}

#[test]
fn unit() -> Result<(), TypeError> {
    let (ast, typ, unbound) = infer(ast::unit())?;

    assert_eq!(typed::unit(), ast);
    assert_eq!(typ::unit(), typ);
    assert!(unbound.is_empty());

    Ok(())
}

#[test]
fn id() -> Result<(), TypeError> {
    let (ast, typ, unbound) = infer(combinators::I())?;

    // id: a -> a
    let a = Var(0);
    assert_eq!(typed::function(0, a, typed::var(0, a)), ast);
    assert_eq!(typ::function(a, a), typ,);
    assert_eq!(set![a], unbound);

    Ok(())
}

#[test]
fn id_call() -> Result<(), TypeError> {
    let (ast, typ, unbound) = infer(ast::call(combinators::I(), ast::unit()))?;

    assert_eq!(
        typed::call(
            typed::function(0, typ::unit(), typed::var(0, typ::unit())),
            typed::unit(),
            typ::unit()
        ),
        ast
    );
    assert_eq!(typ::unit(), typ);
    assert!(unbound.is_empty());

    Ok(())
}

#[test]
fn k() -> Result<(), TypeError> {
    let (ast, typ, unbound) = infer(combinators::K())?;

    // K: a -> b -> a
    let a = Var(0);
    let b = Var(1);
    assert_eq!(
        typed::function(0, a, typed::function(1, b, typed::var(0, a))),
        ast
    );
    assert_eq!(typ::function(a, typ::function(b, a)), typ);
    assert_eq!(set![a, b], unbound);

    Ok(())
}

#[test]
fn k_partial() -> Result<(), TypeError> {
    let (ast, typ, unbound) = infer(ast::call(combinators::K(), ast::unit()))?;

    // K: a -> b -> a
    // K (): b -> ()
    let b = Var(1);
    assert_eq!(
        typed::call(
            typed::function(
                0,
                typ::unit(),
                typed::function(1, b, typed::var(0, typ::unit()))
            ),
            typed::unit(),
            typ::function(b, typ::unit())
        ),
        ast
    );
    assert_eq!(typ::function(b, typ::unit()), typ);
    assert_eq!(set![b], unbound);

    Ok(())
}

#[test]
fn k_total() -> Result<(), TypeError> {
    let mut c = combinators::new();
    let (ast, typ, unbound) =
        infer(ast::call(ast::call(c.K(), ast::unit()), c.I()))?;

    // I: a -> a
    // K: b -> c -> b
    // K () I: ()
    let a = Var(0);
    assert_eq!(
        typed::call(
            typed::call(
                typed::function(
                    0,
                    typ::unit(),
                    typed::function(
                        1,
                        typ::function(a, a),
                        typed::var(0, typ::unit())
                    )
                ),
                typed::unit(),
                typ::function(typ::function(a, a), typ::unit())
            ),
            typed::function(2, a, typed::var(2, a)),
            typ::unit()
        ),
        ast
    );
    assert_eq!(typ::unit(), typ);
    assert_eq!(set![a], unbound);

    Ok(())
}

#[test]
fn s() -> Result<(), TypeError> {
    let (ast, typ, unbound) = infer(combinators::S())?;

    // S: (a -> b -> c) -> (a -> b) -> a -> c
    let a = Var(2);
    let b = Var(3);
    let c = Var(4);
    assert_eq!(
        typed::function(
            0,
            typ::function(a, typ::function(b, c)),
            typed::function(
                1,
                typ::function(a, b),
                typed::function(
                    2,
                    a,
                    typed::call(
                        typed::call(
                            typed::var(
                                0,
                                typ::function(a, typ::function(b, c))
                            ),
                            typed::var(2, a),
                            typ::function(b, c)
                        ),
                        typed::call(
                            typed::var(1, typ::function(a, b)),
                            typed::var(2, a),
                            b
                        ),
                        c
                    )
                )
            )
        ),
        ast
    );
    assert_eq!(
        typ::function(
            // a -> b -> c
            typ::function(a, typ::function(b, c)),
            typ::function(
                // a -> b
                typ::function(a, b),
                // a -> c
                typ::function(a, c)
            )
        ),
        typ
    );
    assert_eq!(set![a, b, c], unbound);

    Ok(())
}

#[test]
fn sk() -> Result<(), TypeError> {
    let mut c = combinators::new();
    let (_, typ, _) = infer(ast::call(c.S(), c.K()))?;
    let a = Var(0);
    let b = Var(1);
    // In untyped lambda calculus SK returns it's second argument so
    // SK<anything> is equivalent to I. Including types the type of the first
    // argument influences the type inference of the second so if we want a
    // genuine identity function we have to choose a first argument that doesn't
    // impose any constraints
    // SK: (a -> b) -> a -> a
    assert_eq!(typ::function(typ::function(a, b), typ::function(a, a)), typ);
    Ok(())
}

#[test]
fn skk_is_id() -> Result<(), TypeError> {
    let mut c = combinators::new();
    // S: (a -> b -> c) -> (a -> b) -> a -> c
    // K: d -> e -> d
    let (_, typ, _) = infer(ast::call(ast::call(c.S(), c.K()), c.K()))?;
    let e = Var(3);
    assert_eq!(typ::function(e, e), typ);
    Ok(())
}

#[test]
fn sks_is_id_sortof() -> Result<(), TypeError> {
    let mut c = combinators::new();
    // S: (a -> b -> c) -> (a -> b) -> a -> c
    // K: d -> e -> d
    // S K: UNIFY(a -> b -> c = d -> e -> d)
    //      UNIFY(a = d, b = e, c = d)
    //      (d -> e) -> d -> d
    // S K S: UNIFY(d -> e = (a -> b -> c) -> (a -> b) -> a -> b)
    //        UNIFY(d = (a -> b -> c), e = (a -> b) -> a -> b)
    //        (a -> b -> c) -> (a -> b -> c)
    let (_, typ, _) = infer(ast::call(ast::call(c.S(), c.K()), c.S()))?;
    let a = Var(2);
    let b = Var(3);
    let c = Var(4);
    let f = typ::function(a, typ::function(b, c));
    assert_eq!(typ::function(f.dup(), f), typ);
    Ok(())
}

#[test]
fn type_conflict() {
    // In untyped lambda calculus SKS should be an identity function but because
    // of how the type inference interacts with the type of the second S it has
    // the constraint that the argument must be a 2 argument funciton, see other
    // tests
    let mut c = combinators::new();
    let sks = ast::call(ast::call(c.S(), c.K()), c.S());
    let Err(err) = infer(ast::call(sks, ast::unit())) else {
        panic!("Expected error")
    };
    let a = Var(3);
    let b = Var(4);
    let c = Var(5);
    assert_eq!(
        TypeError::IncompatibleTypes(
            mono_typ::unit(),
            mono_typ::function(a, typ::function(b, c))
        ),
        err
    );
}

#[test]
fn y_has_infinite_type() {
    let Err(err) = infer(combinators::Y()) else {
        panic!("Expected an error")
    };
    assert_eq!(
        TypeError::InfiniteType(Var(1), mono_typ::function(Var(1), Var(2))),
        err
    );
}
