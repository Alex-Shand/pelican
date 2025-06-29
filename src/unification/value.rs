use ena::unify::UnifyValue;

use super::Unify;

#[derive(Debug)]
pub(crate) struct Value<T: Unify>(pub(crate) T);

impl<T: Unify> Clone for Value<T> {
    fn clone(&self) -> Self {
        Self(self.0.dup())
    }
}

impl<T: Unify> UnifyValue for Value<T> {
    type Error = <T as Unify>::Error;

    fn unify_values(left: &Self, right: &Self) -> Result<Self, Self::Error> {
        Ok(Value(Unify::merge(left.0.dup(), right.0.dup())?))
    }
}
