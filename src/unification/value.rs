use ena::unify::UnifyValue;
use value_type::value_type;

use super::Unify;

#[value_type]
pub(crate) struct Value<T>(pub(crate) T);

impl<T: Unify> UnifyValue for Value<T> {
    type Error = <T as Unify>::Error;

    fn unify_values(left: &Self, right: &Self) -> Result<Self, Self::Error> {
        Ok(Value(Unify::merge(&left.0, &right.0)?))
    }
}
