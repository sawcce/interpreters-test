use std::mem::transmute;

use crate::expr::{Binding, Expr, Operator};
use crate::implementations::*;

use self::operations::native_op_neq;

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub enum Value {
    Nil,
    Boolean(bool),
    Float(f64),
}

#[derive(Debug, Clone, Copy)]
enum Hint {
    Return = 1001,
    Break = 1002,
    While = 1003,
}

impl Value {
    pub fn truthy(&self) -> bool {
        match self {
            Value::Nil => false,
            Value::Boolean(b) => *b,
            Value::Float(f) => *f == 1.0,
        }
    }
}

/// WARNING! You have to be extremely careful when calling
/// transmute on Operation, if T doesn't correspond to the
/// actual T type it will cause segmentation faults.
/// Hence you can't use T=() to mean a generic Operation
/// and discard the result.
pub struct Operation<T = Value>(unsafe fn(&mut CallContext) -> T);

impl<T> Operation<T> {
    #[inline]
    pub unsafe fn call(self, ctx: &mut CallContext) -> T {
        unsafe { self.0(ctx) }
    }
}

#[derive(Debug, Clone)]
pub struct Tape {
    tape: *const u64,
    size: usize,
    pub offset: usize,
}

impl Tape {
    pub fn new(tape: *const u64, size: usize) -> Self {
        Self {
            tape,
            size,
            offset: 0,
        }
    }

    pub fn get_next(&mut self) -> u64 {
        //println!("Getting next {:?}!", self);
        if self.offset >= self.size {
            panic!(
                "End of tape reached: max offset is: {}, tried to access: {}",
                self.size - 1,
                self.offset + 1
            );
        }

        self.offset += 1;

        unsafe {
            let val = self.tape.read();

            self.tape = self.tape.add(1);
            return val;
        };
    }

    pub unsafe fn peek(&mut self) -> u64 {
        self.tape.add(1).read()
    }

    pub unsafe fn read(&self) -> u64 {
        self.tape.read()
    }

    pub unsafe fn get_next_u128(&mut self) -> u128 {
        if self.offset >= self.size {
            panic!("End of tape reached");
        }

        self.offset += 2;

        let ptr = self.tape as *const u128;
        let value = ptr.clone().read();

        self.tape = self.tape.add(2);
        value
    }

    pub fn get_next_float(&mut self) -> f64 {
        unsafe { transmute(self.get_next()) }
    }

    pub fn get_next_func<T>(&mut self) -> Operation<T> {
        let func = unsafe { transmute(self.get_next()) };
        Operation(func)
    }

    pub fn save(&self) -> (usize, *const u64) {
        (self.offset, self.tape)
    }

    pub unsafe fn restore(&mut self, (offset, old): (usize, *const u64)) {
        self.tape = old;
        self.offset = offset;
    }

    pub unsafe fn skip(&mut self, amount: usize) {
        self.tape = self.tape.add(amount);
        self.offset += amount;
    }

    pub unsafe fn move_to(&mut self, dest: usize) {
        let start = self.tape.sub(self.offset);
        self.tape = start.add(dest);
    }

    pub unsafe fn debug(&mut self) {
        let mut ptr = self.tape.sub(self.offset - 1).clone();

        for i in 0..self.size - 10 {
            println!("{i}: {}", *ptr);
            ptr = ptr.add(1);
        }
    }
}

#[derive(Debug, Clone)]
pub struct ImCompiler {
    pub globals: Vec<String>,
    pub future_tape: Vec<u64>,
}

impl ImCompiler {
    pub fn new() -> Self {
        Self {
            globals: Vec::new(),
            future_tape: Vec::new(),
        }
    }

    pub fn push(&mut self, value: u64) {
        self.future_tape.push(value);
    }

    pub fn constant_get_or_def(&mut self, name: impl ToString) -> usize {
        let name = name.to_string();

        let idx = if !self.globals.contains(&name) {
            self.globals.push(name.clone());
            self.globals.len() - 1
        } else {
            self.globals.iter().position(|x| x == &name).unwrap()
        };

        idx
    }

    pub fn compile_expr(&mut self, expr: Expr) {
        match expr {
            Expr::Boolean(b) => {
                if b {
                    self.push(unsafe { transmute(Operation(literals::tr) as Operation<Value>) });
                } else {
                    self.push(unsafe { transmute(Operation(literals::fl) as Operation<Value>) });
                }
            }
            Expr::Float(x) => {
                self.push(unsafe { transmute(Operation(literals::float) as Operation<Value>) });
                self.push(unsafe { transmute(x) });
            }

            Expr::Var(binding) => match binding {
                Binding::Global(name) => {
                    unsafe fn var(ctx: &mut CallContext) -> Value {
                        let idx = ctx.tape.get_next();
                        return ctx.globals[idx as usize].clone();
                    }

                    let idx = self.constant_get_or_def(name) as u64;

                    self.push(unsafe { transmute(Operation(var) as Operation<Value>) });
                    self.push(idx);
                }
            },

            Expr::Assign(binding, value) => match binding {
                Binding::Global(name) => {
                    unsafe fn assign(ctx: &mut CallContext) -> Value {
                        let idx = ctx.tape.get_next();
                        let value = ctx.tape.get_next_func::<Value>();

                        ctx.globals[idx as usize] = transmute(value.call(ctx));

                        Value::Nil
                    }

                    self.push(unsafe { transmute(Operation(assign) as Operation<Value>) });
                    let idx = self.constant_get_or_def(name);
                    self.push(idx as u64);
                    self.compile_expr(*value);
                }
            },

            Expr::Return(value) => {
                self.push(Hint::Return as u64);
                self.compile_expr(*value);
            }

            Expr::Block(statements) => {
                let instr_idx = self.future_tape.len();
                self.push(0);
                // self.push(unsafe { transmute(Operation(block) as Operation<Value>) });
                let next_instr = self.future_tape.len();
                self.push(0);

                let (mut has_return, mut has_while) = (false, false);

                for statement in statements {
                    if let Expr::Return(_) = statement {
                        has_return = true;
                        break;
                    } else if let Expr::While(_, _) = statement {
                        has_while = true;
                    }

                    self.compile_expr(statement);
                }

                self.future_tape[instr_idx] = if has_return || has_while {
                    unsafe { transmute(Operation(flow::block_checked) as Operation<Value>) }
                } else {
                    unsafe { transmute(Operation(flow::block) as Operation<Value>) }
                };

                // Explicitely fetching the next instruction's index avoids
                // off by one errors
                self.future_tape[next_instr] = self.future_tape.len() as u64;
            }

            Expr::While(cond, body) => {
                self.push(Hint::While as u64);
                self.push(unsafe {
                    transmute(Operation(flow::while_loop) as Operation<Option<Value>>)
                });

                let next_instr = self.future_tape.len();
                self.push(0);

                self.compile_expr(*cond);
                self.compile_expr(*body);

                // Explicitely fetching the next instruction's index avoids
                // off by one errors
                self.future_tape[next_instr] = self.future_tape.len() as u64;
            }

            Expr::BinaryOp(lhs, op, rhs) => {
                let func = match op {
                    Operator::Add => operations::native_op_add,
                    Operator::Sub => operations::native_op_sub,
                    Operator::Mul => operations::native_op_mul,
                    Operator::Div => operations::native_op_div,
                    Operator::Rem => operations::native_op_rem,
                    Operator::Eq => operations::native_op_eq,
                    Operator::Neq => operations::native_op_neq,
                    Operator::Gt => operations::native_op_gt,
                    Operator::Gte => operations::native_op_gte,
                    Operator::Lt => operations::native_op_lt,
                    Operator::Lte => operations::native_op_lte,
                };

                self.future_tape
                    .push(unsafe { transmute(Operation(func) as Operation<Value>) });
                self.compile_expr(*lhs);
                self.compile_expr(*rhs);
            }

            Expr::Add(_, _) => {}
        }
    }
}

#[derive(Debug, Clone)]
pub struct CallContext {
    pub tape: Tape,
    stack: Vec<Value>,
    globals: Vec<Value>,
}

impl CallContext {
    pub fn new(tape: *const u64, size: usize, globals_amt: usize) -> Self {
        Self {
            tape: Tape::new(tape, size),
            stack: Vec::new(),
            globals: vec![Value::Float(0.0); globals_amt],
        }
    }

    pub fn execute(&mut self) -> Value {
        unsafe { self.tape.get_next_func::<Value>().call(self) }
    }
}

#[test]
pub fn nested() {
    let prog = Expr::Block(vec![
        Binding::Global("test".into())
            .assign(Expr::Block(vec![
                Expr::Return(Expr::Float(10.0).into()).into()
            ]))
            .into(),
        Binding::Global("test2".into())
            .assign(Expr::Block(vec![Expr::While(
                Expr::Boolean(true).into(),
                Expr::Return(Expr::Float(50.0).into()).into(),
            )
            .into()]))
            .into(),
    ]);

    let mut compiler = ImCompiler::new();
    compiler.compile_expr(prog);

    println!("Compiler: {compiler:?}");

    let mut context = CallContext::new(
        compiler.future_tape.as_ptr(),
        compiler.future_tape.len(),
        compiler.globals.len(),
    );
    context.execute();

    println!("End ctx: {context:?}");
}

#[test]
pub fn tape_test() {
    let expr = Expr::Block(vec![
        // Expr::Return(Expr::Float(0.0).into()),
        Binding::Global("x".into()).assign(Expr::Float(100_000_000.0)),
        Expr::While(
            Expr::BinaryOp(
                Expr::Var(Binding::Global("x".into())).into(),
                Operator::Gt,
                Expr::Float(0.0).into(),
            )
            .into(),
            Binding::Global("x".into())
                .assign(Expr::BinaryOp(
                    Expr::Var(Binding::Global("x".into())).into(),
                    Operator::Sub,
                    Expr::Float(1.0).into(),
                ))
                .into(),
        ),
        Expr::Return(Binding::Global("x".into()).var().into()),
    ]);

    let mut compiler = ImCompiler::new();
    compiler.compile_expr(expr);

    // println!("Compiler: {compiler:?}");

    let mut context = CallContext::new(
        compiler.future_tape.as_ptr(),
        compiler.future_tape.len(),
        compiler.globals.len(),
    );
    context.execute();

    assert_eq!(context.globals[0], Value::Float(0.0));
}
