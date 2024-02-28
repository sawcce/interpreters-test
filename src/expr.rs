#[derive(Debug, Clone)]
pub enum Binding {
    Global(String),
}

#[derive(Debug, Clone)]
pub enum Expr {
    Float(f64),
    Assign(Binding, Box<Expr>),
    Add(Box<Expr>, Box<Expr>),
    Block(Vec<Expr>),
    Var(Binding),
}

impl Binding {
    pub fn assign(self, value: impl Into<Box<Expr>>) -> Expr {
        Expr::Assign(self, value.into())
    }
}

impl Expr {
    pub fn global(name: impl ToString) -> Self {
        Self::Var(Binding::Global(name.to_string()))
    }
    
    pub fn add(self, rhs: impl Into<Box<Expr>>) -> Self {
        Expr::Add(Box::new(self), rhs.into())
    }
}