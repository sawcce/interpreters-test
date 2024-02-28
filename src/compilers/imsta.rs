use std::mem::transmute;

use crate::expr::{Binding, Expr};

#[derive(Debug, Clone, Copy)]
enum Value {
    Nil,
    Boolean(bool),
    Float(f64),
}

pub type Operation<T> = unsafe fn(&mut CallContext) -> T;

#[derive(Debug, Clone)]
pub struct Tape {
    tape: *const u64,
    size: usize,
    offset: usize,
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
        if self.offset == self.size {
            panic!("End of tape reached");
        }

        self.offset += 1;

        unsafe {
            let val = self.tape.read();

            self.tape = self.tape.add(1);
            return val;
        };
    }

    pub unsafe fn get_next_u128(&mut self) -> u128 {
        if self.offset == self.size {
            panic!("End of tape reached");
        }

        self.offset += 2;

        let ptr = self.tape as *const u128;
        let value = ptr.read();

        self.tape = self.tape.add(2);
        value
    }

    pub fn get_next_float(&mut self) -> f64 {
        unsafe { transmute(self.get_next()) }
    }

    pub fn get_next_func<T>(&mut self) -> unsafe fn(&mut CallContext) -> T {
        let func = unsafe { transmute(self.get_next()) };
        func
    }
}

#[derive(Debug, Clone)]
pub struct ImCompiler {
    globals: Vec<String>,
    future_tape: Vec<u64>,
}

impl ImCompiler {
    pub fn new() -> Self {
        Self {
            globals: Vec::new(),
            future_tape: Vec::new(),
        }
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
        println!("Expr: {expr:?}");

        match expr {
            Expr::Float(x) => {
                unsafe fn float(ctx: &mut CallContext) -> Value {
                    Value::Float(transmute(ctx.tape.get_next()))
                }

                self.future_tape
                    .push(unsafe { transmute(float as Operation<Value>) });
                self.future_tape.push(unsafe { transmute(x) });
            }

            Expr::Var(binding) => match binding {
                Binding::Global(name) => {
                    unsafe fn var(ctx: &mut CallContext) -> u128 {
                        let idx = ctx.tape.get_next();
                        return transmute(ctx.globals[idx as usize].clone());
                    }

                    let idx = self.constant_get_or_def(name) as u64;

                    self.future_tape
                        .push(unsafe { transmute(var as Operation<u128>) });
                    self.future_tape.push(idx);
                }
            },

            Expr::Assign(binding, value) => match binding {
                Binding::Global(name) => {
                    unsafe fn assign(ctx: &mut CallContext) {
                        println!("Assign!");
                        let idx = ctx.tape.get_next();
                        let value = ctx.tape.get_next_func::<Value>();

                        ctx.globals[idx as usize] = transmute(value(ctx));
                    }

                    self.future_tape
                        .push(unsafe { transmute(assign as Operation<()>) });
                    let idx = self.constant_get_or_def(name);
                    self.future_tape.push(idx as u64);
                    self.compile_expr(*value);
                }
            },
            _ => {}
        }
    }
}

#[derive(Debug, Clone)]
pub struct CallContext {
    tape: Tape,
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

    pub fn execute(&mut self) {
        while self.tape.offset < self.tape.size {
            let val = self.tape.get_next();
            let func: Operation<()> = unsafe { std::mem::transmute(val) };
            unsafe { (func)(self) };
        }
    }
}

#[test]
pub fn tape_test() {
    let expr = Binding::Global("x".into()).assign(Expr::Float(100.0));

    let mut compiler = ImCompiler::new();
    compiler.compile_expr(expr);

    println!("Compiler: {compiler:?}");

    let mut context = CallContext::new(
        compiler.future_tape.as_ptr(),
        compiler.future_tape.len(),
        compiler.globals.len(),
    );
    context.execute();

    println!("Context: {:?}", context);
}
