pub mod expr;
pub mod compilers;

pub use compilers::*;

#[cfg(test)]
mod tests {
    use super::expr::*;
    use super::compilers::*;

    #[test]
    fn add() {
        let ir = Expr::Block(vec![
            Binding::Global("x".to_string()).assign(Expr::Float(5.0)),
            Binding::Global("y".to_string()).assign(Expr::Float(1.0)),
            Binding::Global("sum".to_string()).assign(Expr::global("x").add(Expr::global("y"))),
        ]);

        let program = ClosureCompiler::compile(ir);

        program();
    }
}