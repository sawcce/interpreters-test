use criterion::{black_box, criterion_group, criterion_main, Criterion};
use interp_test::imsta::*;
use interp_test::expr::*;

fn count_native() {
    let mut i = black_box(10_000_000);

    while i > 0 {
        i -= black_box(1);
    }
}

fn count_tape() {
    let expr = Expr::Block(vec![
        Binding::Global("x".into()).assign(Expr::Float(100_000_000.0)),
        Expr::While(
            Expr::BinaryOp(
                Expr::Var(Binding::Global("x".into())).into(),
                Operator::Gt,
                Expr::Float(0.0).into(),
            )
            .into(),
            Binding::Global("x".into()).assign(
                Expr::BinaryOp(
                    Expr::Var(Binding::Global("x".into())).into(),
                    Operator::Sub,
                    Expr::Float(1.0).into(),
                ),
            ).into(),
        ),
    ]);

    let mut compiler = ImCompiler::new();
    compiler.compile_expr(expr);

    //println!("Compiler: {compiler:?}");

    let mut context = CallContext::new(
        compiler.future_tape.as_ptr(),
        compiler.future_tape.len(),
        compiler.globals.len(),
    );
    black_box(context.execute());

    // println!("Context: {:?}", context);
}

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("count_native(10M)", |b| b.iter(|| black_box(count_native())));
    c.bench_function("count(10M)", |b| b.iter(|| black_box(count_tape())));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);