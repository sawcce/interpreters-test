use crate::expr::{Binding, Expr};

pub struct ClosureCompiler {
    constants: Vec<String>,
}

#[derive(Debug, Clone, Copy)]
enum Value {
    Float(f64),
    Nil,
}

#[derive(Debug)]
pub struct CallContext<'a> {
    stack: Vec<Value>,
    constants: &'a mut [Value],
}

impl ClosureCompiler {
    pub fn compile<'a>(program: Expr) -> Box<dyn Fn() + 'a> {
        let mut context = ClosureCompiler {
            constants: Vec::new(),
        };

        let closure = Self::compile_expr(&mut context, program);

        Box::new(move || {
            let mut constants = vec![Value::Nil; context.constants.len()];

            let mut ctx = CallContext {
                stack: Vec::new(),
                constants: constants.as_mut_slice(),
            };

            closure(&mut ctx);
        })
    }

    pub fn constant_get_or_def(&mut self, name: impl ToString) -> usize {
        let name = name.to_string();

        let idx = if !self.constants.contains(&name) {
            self.constants.push(name.clone());
            self.constants.len() - 1
        } else {
            self.constants.iter().position(|x| x == &name).unwrap()
        };

        idx
    }

    pub fn compile_expr<'a>(&mut self, expr: Expr) -> Box<dyn Fn(&mut CallContext) + 'a> {
        let expr: Box<dyn Fn(&mut CallContext) + 'a> = match expr {
            Expr::Var(binding) => match binding {
                Binding::Global(ref name) => {
                    let idx = self.constant_get_or_def(name);

                    Box::new(move |ctx: &mut CallContext| {
                        ctx.stack.push(ctx.constants[idx]);
                    })
                }
            },

            Expr::Float(n) => Box::new(move |ctx: &mut CallContext| {
                ctx.stack.push(Value::Float(n));
            }),

            Expr::Assign(binding, value) => match binding {
                Binding::Global(ref name) => {
                    let idx = self.constant_get_or_def(name);
                    let val = self.compile_expr(*value);

                    Box::new(move |ctx| {
                        val(ctx);
                        let val = ctx.stack.pop().unwrap();
                        ctx.constants[idx] = val;
                    })
                }
            },

            Expr::Add(lhs, rhs) => {
                let lhs = self.compile_expr(*lhs);
                let rhs = self.compile_expr(*rhs);

                Box::new(move |ctx| {
                    lhs(ctx);
                    rhs(ctx);

                    let rhs = ctx.stack.pop().unwrap();
                    let lhs = ctx.stack.pop().unwrap();

                    let val = match (lhs, rhs) {
                        (Value::Float(x), Value::Float(y)) => x + y,
                        _ => panic!("Add operation invalid!"),
                    };

                    ctx.stack.push(Value::Float(val));
                })
            }

            Expr::Block(instrs) => {
                let closures: Vec<Box<dyn Fn(&mut CallContext)>> = instrs
                    .iter()
                    .map(|e| self.compile_expr(e.clone()))
                    .collect();

                Box::new(move |ctx: &mut CallContext| {
                    for i in 0..closures.len() {
                        let c = closures.get(i).unwrap();
                        c(ctx);
                    }
                })
            }

            _ => Box::new(|_: &mut CallContext| {}),
        };

        expr
    }
}
