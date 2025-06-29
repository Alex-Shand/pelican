use super::implementation::Type;
use crate::unification::{ValueOrVar, Var};

impl From<Type> for ValueOrVar<Type> {
    fn from(typ: Type) -> Self {
        Self::Value(typ)
    }
}

impl From<Var> for ValueOrVar<Type> {
    fn from(var: Var) -> Self {
        Self::Var(var)
    }
}

pub(super) mod ast {
    use trivial::TrivialBox;

    use crate::unification::tests::lambda::implementation::Ast;

    pub(crate) fn unit() -> Ast {
        Ast::Unit
    }

    pub(crate) fn var(id: usize) -> Ast {
        Ast::Var(id)
    }

    pub(crate) fn function(arg: usize, body: Ast) -> Ast {
        Ast::Function {
            arg,
            body: TrivialBox::new(body),
        }
    }

    pub(crate) fn call(subject: Ast, arg: Ast) -> Ast {
        Ast::Call {
            subject: TrivialBox::new(subject),
            arg: TrivialBox::new(arg),
        }
    }
}

pub(super) mod mono_typ {
    use trivial::TrivialBox;

    use crate::unification::{ValueOrVar, tests::lambda::implementation::Type};

    pub(crate) fn unit() -> Type {
        Type::Unit
    }

    pub(crate) fn function(
        arg: impl Into<ValueOrVar<Type>>,
        ret: impl Into<ValueOrVar<Type>>,
    ) -> Type {
        Type::Function {
            arg: TrivialBox::new(arg.into()),
            ret: TrivialBox::new(ret.into()),
        }
    }
}

pub(super) mod typ {
    use super::mono_typ;
    use crate::unification::{ValueOrVar, tests::lambda::implementation::Type};

    pub(crate) fn unit() -> ValueOrVar<Type> {
        mono_typ::unit().into()
    }

    pub(crate) fn function(
        arg: impl Into<ValueOrVar<Type>>,
        ret: impl Into<ValueOrVar<Type>>,
    ) -> ValueOrVar<Type> {
        mono_typ::function(arg, ret).into()
    }
}

pub(super) mod typed {
    use crate::unification::{
        ValueOrVar,
        tests::lambda::implementation::{Type, TypedAst},
    };

    pub(crate) fn unit() -> TypedAst {
        TypedAst::Unit
    }

    pub(crate) fn var(id: usize, typ: impl Into<ValueOrVar<Type>>) -> TypedAst {
        TypedAst::Var(id, typ.into())
    }

    pub(crate) fn function(
        arg: usize,
        arg_type: impl Into<ValueOrVar<Type>>,
        body: TypedAst,
    ) -> TypedAst {
        TypedAst::Function {
            arg,
            arg_type: arg_type.into(),
            body: Box::new(body),
        }
    }

    pub(crate) fn call(
        subject: TypedAst,
        arg: TypedAst,
        typ: impl Into<ValueOrVar<Type>>,
    ) -> TypedAst {
        TypedAst::Call {
            subject: Box::new(subject),
            arg: Box::new(arg),
            typ: typ.into(),
        }
    }
}

#[allow(non_snake_case)]
pub(super) mod combinators {
    use trivial::Trivial as _;

    use super::ast::*;
    use crate::unification::tests::lambda::implementation::Ast;

    pub(crate) fn new() -> Combinators {
        Combinators { id: 0 }
    }

    pub(crate) fn I() -> Ast {
        new().I()
    }

    pub(crate) fn K() -> Ast {
        new().K()
    }

    pub(crate) fn S() -> Ast {
        new().S()
    }

    pub(crate) fn Y() -> Ast {
        new().Y()
    }

    pub(crate) struct Combinators {
        id: usize,
    }

    impl Combinators {
        fn next_id(&mut self) -> usize {
            let id = self.id;
            self.id += 1;
            id
        }

        pub(crate) fn I(&mut self) -> Ast {
            let arg = self.next_id();
            function(arg, var(arg))
        }

        pub(crate) fn K(&mut self) -> Ast {
            let a = self.next_id();
            let b = self.next_id();
            function(a, function(b, var(a)))
        }

        pub(crate) fn S(&mut self) -> Ast {
            let x = self.next_id();
            let y = self.next_id();
            let z = self.next_id();
            // Sxyz == xz(yz)
            function(
                x,
                function(
                    y,
                    function(
                        z,
                        call(call(var(x), var(z)), call(var(y), var(z))),
                    ),
                ),
            )
        }

        pub(crate) fn Y(&mut self) -> Ast {
            let f = self.next_id();
            let x = self.next_id();
            let inner = function(x, call(var(f), call(var(x), var(x))));
            function(f, call(inner.dup(), inner))
        }
    }
}
