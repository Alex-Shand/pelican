use std::{
    cmp::Ordering,
    fmt,
    hash::{Hash, Hasher},
    marker::PhantomData,
};

use ena::unify::UnifyKey;
use value_type::value_type;

use super::{Unify, value::Value};

/// Unification variable
#[value_type(Copy)]
pub struct Var(pub(crate) u32);

impl fmt::Display for Var {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

impl Var {
    pub(crate) fn annotate<T: Unify>(self) -> TypedVar<T> {
        TypedVar(self.0, PhantomData)
    }
}

#[derive(Clone)]
pub(crate) struct TypedVar<T: Unify>(u32, PhantomData<T>);

impl<T: Unify> TypedVar<T> {
    pub(crate) fn erase(self) -> Var {
        Var(self.0)
    }
}

impl<T: Unify> fmt::Debug for TypedVar<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.erase(), f)
    }
}

impl<T: Unify> Copy for TypedVar<T> {}

impl<T: Unify> PartialEq for TypedVar<T> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}
impl<T: Unify> Eq for TypedVar<T> {}

impl<T: Unify> Ord for TypedVar<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0)
    }
}
impl<T: Unify> PartialOrd for TypedVar<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<T: Unify> Hash for TypedVar<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl<T: Unify> UnifyKey for TypedVar<T> {
    type Value = Option<Value<T>>;

    fn index(&self) -> u32 {
        self.0
    }

    fn from_index(u: u32) -> Self {
        Self(u, PhantomData)
    }

    fn tag() -> &'static str {
        "Var"
    }
}
