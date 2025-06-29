use std::collections::{HashMap, HashSet};

use trivial::{Claim as _, Trivial, TrivialBox};

use crate::{
    map::Map,
    unification::{Table, Unify, ValueOrVar, Var},
};

// Input for the typechecker, untyped lambda calculus-ish
//
// Contains variables, Single argument functions, Function Call and a Unit value
#[derive(Debug, Trivial)]
pub(super) enum Ast {
    Unit,
    Var(usize),
    Function {
        arg: usize,
        body: TrivialBox<Ast>,
    },
    Call {
        subject: TrivialBox<Ast>,
        arg: TrivialBox<Ast>,
    },
}

// Output, Identical except we now know the type of everything
#[derive(Debug, PartialEq)]
pub(super) enum TypedAst {
    // Unit's type is always known, so no need to record it
    Unit,
    Var(usize, ValueOrVar<Type>),
    // No need to store the body type since body is now a TypedAst
    Function {
        arg: usize,
        arg_type: ValueOrVar<Type>,
        body: Box<TypedAst>,
    },
    // subject will always have a function type if typechecking passes and the
    // overall type of a call will be the return type of that function. Storing
    // it anyway because that would require jumping through a few hoops to get
    // to
    Call {
        subject: Box<TypedAst>,
        arg: Box<TypedAst>,
        typ: ValueOrVar<Type>,
    },
}

impl TypedAst {
    fn substitute(self, types: &HashMap<Var, ValueOrVar<Type>>) -> Self {
        match self {
            TypedAst::Unit => TypedAst::Unit,
            TypedAst::Var(name, typ) => {
                TypedAst::Var(name, typ.resolve(types, Type::walk))
            }
            TypedAst::Function {
                arg,
                arg_type,
                body,
            } => TypedAst::Function {
                arg,
                arg_type: arg_type.resolve(types, Type::walk),
                body: Box::new(body.substitute(types)),
            },
            TypedAst::Call { subject, arg, typ } => TypedAst::Call {
                subject: Box::new(subject.substitute(types)),
                arg: Box::new(arg.substitute(types)),
                typ: typ.resolve(types, Type::walk),
            },
        }
    }
}

// Types
#[derive(Debug, PartialEq, Eq, Trivial)]
pub(super) enum Type {
    Unit,
    Function {
        arg: TrivialBox<ValueOrVar<Self>>,
        ret: TrivialBox<ValueOrVar<Self>>,
    },
}

impl Type {
    // Check if a type contains a specific unification variable. Necessary to
    // avoid infinite recursion while unifiying
    fn contains(&self, var: Var) -> bool {
        match self {
            // Unit contains no type variables
            Type::Unit => false,
            Type::Function { arg, ret } => {
                match &**arg {
                    // If the argument is a variable and that variable is the one we
                    // want return true immediately
                    ValueOrVar::Var(v) => {
                        if *v == var {
                            return true;
                        }
                    }
                    // If it's a type return true if that type contains the
                    // variable
                    ValueOrVar::Value(ty) => {
                        if ty.contains(var) {
                            return true;
                        }
                    }
                }
                // Likewise with the return type
                match &**ret {
                    ValueOrVar::Var(v) => {
                        if *v == var {
                            return true;
                        }
                    }
                    ValueOrVar::Value(ty) => {
                        if ty.contains(var) {
                            return true;
                        }
                    }
                }
                false
            }
        }
    }

    fn walk(typ: Type, types: &HashMap<Var, ValueOrVar<Type>>) -> Type {
        match typ {
            Type::Unit => Type::Unit,
            Type::Function { arg, ret } => Type::Function {
                arg: TrivialBox::new(arg.take().resolve(types, Self::walk)),
                ret: TrivialBox::new(ret.take().resolve(types, Self::walk)),
            },
        }
    }
}

// Type errors
#[derive(Debug, PartialEq)]
pub(super) enum TypeError {
    IncompatibleTypes(Type, Type),
    InfiniteType(Var, Type),
}

impl Unify for Type {
    type Error = TypeError;

    fn unify(
        left: ValueOrVar<Type>,
        right: ValueOrVar<Type>,
        unifier: &mut crate::unification::Unifier<Self>,
    ) -> Result<(), Self::Error> {
        let mut unifier = Unifier(unifier);
        unifier.unify_typ(left, right)
    }

    // We only allow concrete types to unify if they are equal
    fn merge(left: Self, right: Self) -> Result<Self, Self::Error> {
        if left != right {
            return Err(TypeError::IncompatibleTypes(left, right));
        }
        Ok(left)
    }
}

// Wrapper for the unifier provided by Pelican. Adds methods that know how to
// deal with the Type enum
struct Unifier<'a>(&'a mut crate::unification::Unifier<Type>);

impl Unifier<'_> {
    // Normalize a type
    fn normalize(&mut self, typ: ValueOrVar<Type>) -> ValueOrVar<Type> {
        match typ {
            // Unit cannot be simplified any further
            ValueOrVar::Value(Type::Unit) => ValueOrVar::Value(Type::Unit),
            // Functions are piecewise normalized
            ValueOrVar::Value(Type::Function { arg, ret }) => {
                let arg = self.normalize(arg.take());
                let ret = self.normalize(ret.take());
                ValueOrVar::Value(Type::Function {
                    arg: TrivialBox::new(arg),
                    ret: TrivialBox::new(ret),
                })
            }
            // To normalize a variable we probe the unifier. This either returns
            // a concrete value, in which case we normalize it, or a (possibly
            // different) variable if we haven't found a concrete type yet
            ValueOrVar::Var(var) => match self.0.probe(var) {
                var @ ValueOrVar::Var(_) => var,
                typ @ ValueOrVar::Value(_) => self.normalize(typ),
            },
        }
    }

    fn unify_typ(
        &mut self,
        left: ValueOrVar<Type>,
        right: ValueOrVar<Type>,
    ) -> Result<(), TypeError> {
        match (self.normalize(left), self.normalize(right)) {
            // Two unit types unify
            (ValueOrVar::Value(Type::Unit), ValueOrVar::Value(Type::Unit)) => {
                Ok(())
            }
            // Function types unify if their argument and return types unify
            (
                ValueOrVar::Value(Type::Function {
                    arg: left_arg,
                    ret: left_ret,
                }),
                ValueOrVar::Value(Type::Function {
                    arg: right_arg,
                    ret: right_ret,
                }),
            ) => {
                self.unify_typ(left_arg.take(), right_arg.take())?;
                self.unify_typ(left_ret.take(), right_ret.take())
            }
            (ValueOrVar::Var(left), ValueOrVar::Var(right)) => {
                self.0.unify_var_var(left, right)
            }
            // We can attempt to unify a variable with a concrete type if the
            // variable in question doesn't appear in the type (or normalize
            // will recurse infinitly).
            //
            // If the variable hasn't been resolved to a concrete type yet then
            // this resolves it.
            //
            // If the variable has already been resolved to a concrete type then
            // Type's Unify impl raises an error if that type is different to
            // this one
            (ValueOrVar::Var(v), ValueOrVar::Value(typ))
            | (ValueOrVar::Value(typ), ValueOrVar::Var(v)) => {
                if typ.contains(v) {
                    return Err(TypeError::InfiniteType(v, typ));
                }
                self.0.unify_var_value(v, typ)
            }
            // Any other combination of things doesn't unify. We have dealt with
            // all possible positions a type variable could appear so this case
            // always deals with concrete types
            (ValueOrVar::Value(left), ValueOrVar::Value(right)) => {
                Err(TypeError::IncompatibleTypes(left, right))
            }
        }
    }
}

// Wrapper for Pelican to hold methods spefific to this Ast and Type structure
struct Engine(Table<Type>);

impl Engine {
    fn new() -> Self {
        Self(Table::new())
    }

    // Bottom up type inference
    fn infer(
        &mut self,
        env: Map<usize, ValueOrVar<Type>>,
        ast: Ast,
    ) -> (TypedAst, ValueOrVar<Type>) {
        match ast {
            // Unit is trivially Unit type
            Ast::Unit => (TypedAst::Unit, ValueOrVar::Value(Type::Unit)),
            // A variable is whatever type it has recorded in the environment.
            // We don't deal with the possibility that the variable doesn't
            // exist
            Ast::Var(v) => {
                let typ = env.get(v).unwrap();
                (TypedAst::Var(v, typ.dup()), typ.dup())
            }
            Ast::Function { arg, body } => {
                // Crate a new type variable for the argument type
                let arg_var = self.0.var();
                // Run inference on the body with the argument variable in
                // scope. This gives us a TypedAst for the body and the return
                // type of the function. It will also introduce constraints on
                // the argument variable which we can use to figure out what
                // type it needs to be
                let env = env.update(arg, ValueOrVar::Var(arg_var));
                let (body, ret) = self.infer(env, body.take());
                (
                    TypedAst::Function {
                        arg,
                        arg_type: ValueOrVar::Var(arg_var),
                        body: Box::new(body),
                    },
                    ValueOrVar::Value(Type::Function {
                        arg: TrivialBox::new(ValueOrVar::Var(arg_var)),
                        ret: TrivialBox::new(ret),
                    }),
                )
            }
            Ast::Call { subject, arg } => {
                // Start by figuring out the type of the argument to the call
                let (arg, arg_typ) = self.infer(env.claim(), arg.take());

                // We know the subject must be a function so we make one with
                // the argument type we inferred and a fresh variable for the
                // return type and check the subject top-down
                let ret = self.0.var();
                let typ = ValueOrVar::Value(Type::Function {
                    arg: TrivialBox::new(arg_typ),
                    ret: TrivialBox::new(ValueOrVar::Var(ret)),
                });
                let subject = self.check(env, subject.take(), typ);
                (
                    TypedAst::Call {
                        subject: Box::new(subject),
                        arg: Box::new(arg),
                        typ: ValueOrVar::Var(ret),
                    },
                    ValueOrVar::Var(ret),
                )
            }
        }
    }

    // Top down type checking
    fn check(
        &mut self,
        env: Map<usize, ValueOrVar<Type>>,
        ast: Ast,
        typ: ValueOrVar<Type>,
    ) -> TypedAst {
        match (ast, typ) {
            // Unit trivially checks against itself
            (Ast::Unit, ValueOrVar::Value(Type::Unit)) => TypedAst::Unit,
            // A function can check against a function type ...
            (
                Ast::Function { arg, body },
                ValueOrVar::Value(Type::Function { arg: arg_type, ret }),
            ) => {
                // ... if the body type-checks against the expected return type
                // with the argument bound to the expected argument type
                let env = env.update(arg, arg_type.dup().take());
                let body = self.check(env, body.take(), ret.take());
                TypedAst::Function {
                    arg,
                    arg_type: arg_type.take(),
                    body: Box::new(body),
                }
            }
            // For any other pair we infer a type for the ast fragment then emit
            // a constraint that the expected type matches the one we inferred
            (ast, expected) => {
                let (out, actual) = self.infer(env, ast);
                self.0.constraint(expected, actual);
                out
            }
        }
    }

    fn unify(self) -> Result<HashMap<Var, ValueOrVar<Type>>, TypeError> {
        self.0.unify()
    }
}

pub(super) fn infer(
    ast: Ast,
) -> Result<(TypedAst, ValueOrVar<Type>, HashSet<Var>), TypeError> {
    let mut engine = Engine::new();
    let (ast, typ) = engine.infer(Map::new(), ast);
    let types = engine.unify()?;
    let unbound = types
        .iter()
        .filter_map(|(_, value)| match value {
            ValueOrVar::Value(_) => None,
            ValueOrVar::Var(var) => Some(*var),
        })
        .collect();
    Ok((
        ast.substitute(&types),
        typ.resolve(&types, Type::walk),
        unbound,
    ))
}
