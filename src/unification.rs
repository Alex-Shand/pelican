//! Unification table

use std::{collections::HashMap, fmt::Debug, mem, ops::Range};

use ena::unify::{
    InPlace, InPlaceUnificationTable, Snapshot, UnificationTable,
};
use value_type::value_type;

pub use self::var::Var;
use self::{value::Value, var::TypedVar};

#[cfg(test)]
mod tests;
mod value;
mod var;

/// Defines how to unify two values in the table
pub trait Unify: Debug + Clone {
    /// Error returned if unification fails
    type Error;

    /// Unification strategy.
    ///
    /// This method will be called once for each constraint added to the [`Table`]
    ///
    /// The [`Unifier`] argument provides methods for communicating with the
    /// unification engine
    fn unify(
        left: ValueOrVar<Self>,
        right: ValueOrVar<Self>,
        unifier: &mut Unifier<Self>,
    ) -> Result<(), Self::Error>;

    /// Merge two concrete values.
    ///
    /// If unification tries to unify two sets which have both been resolved to
    /// concrete values, this method is called to produce the new value
    fn merge(left: &Self, right: &Self) -> Result<Self, Self::Error>;
}

/// Unification table
#[expect(missing_debug_implementations)]
pub struct Table<T: Unify> {
    unification_table: InPlaceUnificationTable<TypedVar<T>>,
    clean_snapshot: Snapshot<InPlace<TypedVar<T>>>,
    constraints: Vec<(ValueOrVar<T>, ValueOrVar<T>)>,
}

impl<T: Unify> Default for Table<T> {
    fn default() -> Self {
        let mut unification_table = UnificationTable::new();
        let clean_snapshot = unification_table.snapshot();
        Self {
            unification_table,
            clean_snapshot,
            constraints: Vec::new(),
        }
    }
}

impl<T: Unify> Table<T> {
    /// Constructor
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a fresh unification variable
    pub fn var(&mut self) -> Var {
        self.unification_table.new_key(None).erase()
    }

    /// Add a new constraint to the table
    pub fn constraint(&mut self, left: ValueOrVar<T>, right: ValueOrVar<T>) {
        self.constraints.push((left, right));
    }

    /// Perform unification
    pub fn unify(mut self) -> Result<HashMap<Var, ValueOrVar<T>>, T::Error> {
        let vars = self.get_vars();
        let constraints = mem::take(&mut self.constraints);
        let mut unifier = Unifier(self);
        for (left, right) in constraints {
            T::unify(left, right, &mut unifier)?;
        }
        let mut result = HashMap::new();
        for var in vars {
            let value = unifier.probe(var);
            let _ = result.insert(var, value);
        }
        Ok(result)
    }

    fn get_vars(&self) -> Vec<Var> {
        let Range { start, end } = self
            .unification_table
            .vars_since_snapshot(&self.clean_snapshot);
        let Var(start) = start.erase();
        let Var(end) = end.erase();
        let mut result = Vec::with_capacity((end - start) as usize);
        for i in start..end {
            result.push(Var(i));
        }
        result
    }
}

/// Helper struct provided to [`Unify::unify`]
///
/// Provides methods for performing unification operations
#[expect(missing_debug_implementations)]
pub struct Unifier<T: Unify>(Table<T>);

impl<T: Unify> Unifier<T> {
    /// Look up the current value of a unification variable
    ///
    /// If the variable has been unified with a concrete value already then that
    /// value is returned.
    ///
    /// If the variable has not been unified with a concrete value then a
    /// representative variable is returned, this may not be the same as the one
    /// passed in
    pub fn probe(&mut self, var: Var) -> ValueOrVar<T> {
        let var = var.annotate();
        match self.0.unification_table.probe_value(var) {
            Some(Value(value)) => ValueOrVar::Value(value),
            None => ValueOrVar::Var(self.0.unification_table.find(var).erase()),
        }
    }

    /// Unify two variables
    ///
    /// Unifying two variables has three possible outcomes
    /// * If both variables have never unified with a concrete value one of them
    ///   will resolve to the other from now on.
    /// * If one variable is resolved to a concrete value and the other isn't
    ///   then the unresolved variable will now resolve to the same concrete
    ///   value.
    /// * If both variables are resolved to concrete values then the values's
    ///   [`Unify::merge`] is called to either merge the two values or produce an
    ///   error.
    pub fn unify_var_var(
        &mut self,
        left: Var,
        right: Var,
    ) -> Result<(), T::Error> {
        self.0
            .unification_table
            .unify_var_var(left.annotate(), right.annotate())
    }

    /// Unify a variable with a concrete value
    ///
    /// If the variable has not yet unified with a concrete value this will
    /// always succeed and the variable (plus any others which have unified to
    /// it) will resolve to the value from now on.
    ///
    /// If the variable has unified with a concrete value then the values's
    /// [`Unify::merge`] will be called to either merge the two types or produce
    /// an error
    pub fn unify_var_value(
        &mut self,
        var: Var,
        typ: T,
    ) -> Result<(), T::Error> {
        self.0
            .unification_table
            .unify_var_value(var.annotate(), Some(Value(typ)))
    }
}

/// Wrapper for a concrete value or a unification variable
#[value_type]
pub enum ValueOrVar<T> {
    #[allow(missing_docs)]
    Value(T),
    #[allow(missing_docs)]
    Var(Var),
}

/// Error returned from [`ValueOrVar::resolve_mono`] if the value cannot be
/// resolved to a monomorphic type
#[value_type(Copy)]
#[derive(thiserror::Error)]
#[error("Unresolved unification variable {0}")]
pub struct UnresolvedVariableError(Var);

impl<T: Clone> ValueOrVar<T> {
    /// Resolve a polymorphic value to it's canonical representation based on the
    /// map returned by [`Table::unify`]
    #[must_use]
    pub fn resolve(
        self,
        table: &HashMap<Var, ValueOrVar<T>>,
        walk: impl Fn(T, &HashMap<Var, ValueOrVar<T>>) -> T,
    ) -> Self {
        match self {
            ValueOrVar::Value(value) => ValueOrVar::Value(walk(value, table)),
            ValueOrVar::Var(var) => match &table[&var] {
                ValueOrVar::Value(value) => {
                    ValueOrVar::Value(walk(value.clone(), table))
                }
                ValueOrVar::Var(var) => ValueOrVar::Var(*var),
            },
        }
    }

    /// Resolve a polymorphic value to it's canonical monomorphic representation
    /// based on the type map returned by [`Table::unify`]
    pub fn resolve_mono(
        self,
        types: &HashMap<Var, ValueOrVar<T>>,
        walk: impl Fn(
            T,
            &HashMap<Var, ValueOrVar<T>>,
        ) -> Result<T, UnresolvedVariableError>,
    ) -> Result<T, UnresolvedVariableError> {
        match self {
            ValueOrVar::Value(value) => walk(value, types),
            ValueOrVar::Var(var) => match &types[&var] {
                ValueOrVar::Value(value) => walk(value.clone(), types),
                ValueOrVar::Var(var) => Err(UnresolvedVariableError(*var)),
            },
        }
    }
}
