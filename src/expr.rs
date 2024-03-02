#[derive(Debug, Clone)]
pub enum Binding {
    Global(String),
}

#[derive(Debug, Clone)]
pub enum Expr {
    Float(f64),
    Boolean(bool),
    Assign(Binding, Box<Expr>),
    Add(Box<Expr>, Box<Expr>),
    Block(Vec<Expr>),
    Var(Binding),
    While(Box<Expr>, Box<Expr>),
    BinaryOp(Box<Expr>, Operator, Box<Expr>),
    Return(Box<Expr>),
    Conditional(Box<(Expr, Expr)>, Vec<(Expr, Expr)>, Box<Expr>),
}

#[derive(Debug, Clone, Copy)]
pub enum Operator {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Lt,
    Lte,
    Gt,
    Gte,
    Eq,
    Neq,
}

impl Binding {
    pub fn assign(self, value: impl Into<Box<Expr>>) -> Expr {
        Expr::Assign(self, value.into())
    }

    pub fn var(self) -> Expr {
        Expr::Var(self)
    }
}

impl Expr {
    pub fn global(name: impl ToString) -> Self {
        Self::Var(Binding::Global(name.to_string()))
    }

    pub fn add(self, rhs: impl Into<Box<Expr>>) -> Self {
        Expr::Add(Box::new(self), rhs.into())
    }

    pub fn op(self, operator: Operator, rhs: impl Into<Box<Expr>>) -> Self {
        Expr::BinaryOp(self.into(), operator, rhs.into())
    }
}
