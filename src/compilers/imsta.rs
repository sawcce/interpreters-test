use std::{
    mem::transmute,
    sync::atomic::{AtomicU64, Ordering},
};

use crate::expr::{Binding, Expr, Operator};

static X: AtomicU64 = AtomicU64::new(0);

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

impl Operation<Value> {
    /// A statement that returns a value will divert
    /// control flow, since we can't return from the
    /// routine's caller we need to manually check
    /// if the evaluated expression returns.
    /// => check if the Operation fn pointer is ret.
    /// Only blocks (Expr::Block) should contain
    /// return instructions.
    pub fn returns(&self) -> bool {
        self.0 == ret
    }
}

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

macro_rules! impl_op {
    ($name:ident, $self:ident, $lhs:ident, $op:tt, $rhs:ident) => {{
        unsafe fn $name(ctx: &mut CallContext) -> Value {
            let lhs = ctx.tape.get_next_func::<Value>().call(ctx);
            let rhs = ctx.tape.get_next_func::<Value>().call(ctx);

            impl_apply_op!(lhs, $op, rhs)
        }

        $self
            .future_tape
            .push(unsafe { transmute(Operation($name) as Operation<Value>) });
        $self.compile_expr(*$lhs);
        $self.compile_expr(*$rhs);
    }};
}

macro_rules! impl_apply_arithmetic {
    ($lhs:ident, $op:tt, $rhs:ident) => {{
        if let Value::Float(f_1) = $lhs {
            if let Value::Float(f_2) = $rhs {
                //println!("Test: {f_1} - {f_2}");
                return Value::Float(f_1 $op f_2);
            }
        }

        panic!("Invalid arguments!");
    }};
}

macro_rules! impl_apply_cmp {
    ($lhs:ident, $op:tt, $rhs:ident) => {
       Value::Boolean($lhs $op $rhs)
    };
}

macro_rules! impl_apply_op {
    ($lhs:ident, +, $rhs:ident) => {impl_apply_arithmetic!($lhs, +, $rhs)};
    ($lhs:ident, -, $rhs:ident) => {impl_apply_arithmetic!($lhs, -, $rhs)};
    ($lhs:ident, *, $rhs:ident) => {impl_apply_arithmetic!($lhs, *, $rhs)};
    ($lhs:ident, /, $rhs:ident) => {impl_apply_arithmetic!($lhs, /, $rhs)};
    ($lhs:ident, %, $rhs:ident) => {impl_apply_arithmetic!($lhs, +, $rhs)};
    ($lhs:ident, ==, $rhs:ident) => {impl_apply_cmp!($lhs, ==, $rhs)};
    ($lhs:ident, !=, $rhs:ident) => {impl_apply_cmp!($lhs, !=, $rhs)};
    ($lhs:ident, >, $rhs:ident) => {impl_apply_cmp!($lhs, >, $rhs)};
    ($lhs:ident, >=, $rhs:ident) => {impl_apply_cmp!($lhs, >=, $rhs)};
    ($lhs:ident, <, $rhs:ident) => {impl_apply_cmp!($lhs, <, $rhs)};
    ($lhs:ident, <=, $rhs:ident) => {impl_apply_cmp!($lhs, <=, $rhs)};
}

unsafe fn ret(ctx: &mut CallContext) -> Value {
    ctx.tape.get_next_func::<Value>().call(ctx)
}

macro_rules! loop_body {
    ($ctx:ident, $length:ident) => {
        if 1001 <= $ctx.tape.read() && $ctx.tape.read() <= 2000 as u64 {
            let v = $ctx.tape.read();
            $ctx.tape.skip(1);
            println!("Test: {v}");
            match v {
                1001 => {
                    let value = $ctx.tape.get_next_func::<Value>().call($ctx);
                    $ctx.tape.skip($length as usize - 1);
                    return value.into();
                }
                1003 => {
                    let value = $ctx.tape.get_next_func::<Option<Value>>().call($ctx);
                    println!("Value: {value:?}");
                    if let Some(x) = value {
                        $ctx.tape.skip($length as usize - 1);
                        return x.into();
                    }
                    continue;
                }
                x => panic!("Invalid block hint: {x}"),
            }
        }
    };
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
                unsafe fn tr(_: &mut CallContext) -> Value {
                    Value::Boolean(true)
                }

                unsafe fn fl(_: &mut CallContext) -> Value {
                    Value::Boolean(false)
                }

                if b {
                    self.push(unsafe {transmute(Operation(tr) as Operation<Value>)});
                } else {
                    self.push(unsafe {transmute(Operation(fl) as Operation<Value>)});
                }
            },
            Expr::Float(x) => {
                unsafe fn float(ctx: &mut CallContext) -> Value {
                    //println!("float => _____");
                    Value::Float(transmute(ctx.tape.get_next()))
                }

                self.push(unsafe { transmute(Operation(float) as Operation<Value>) });
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
                unsafe fn block(ctx: &mut CallContext) -> Value {
                    let length = ctx.tape.get_next() as usize;
                    let start = ctx.tape.offset;
                    println!(
                        "Block length {:?} {:?} => {}",
                        length,
                        ctx.tape,
                        start + length
                    );

                    while ctx.tape.offset < start + length - 1 {
                        loop_body!(ctx, length);

                        let func = ctx.tape.get_next_func::<Value>();
                        func.call(ctx);
                    }

                    Value::Nil
                }

                self.push(unsafe { transmute(Operation(block) as Operation<Value>) });
                let fix_idx = self.future_tape.len();
                self.push(0);

                for statement in statements {
                    // Be sure to handle Return statements
                    self.compile_expr(statement);
                }

                self.future_tape[fix_idx] = (self.future_tape.len() - fix_idx) as u64;
            }

            // TODO: Enable a return statement inside while
            Expr::While(cond, body) => {
                unsafe fn l(ctx: &mut CallContext) -> Option<Value> {
                    println!("While");
                    let size: u64 = ctx.tape.get_next();
                    let tape_ptr = ctx.tape.save();

                    let pk = ctx.tape.peek();
                    println!("Peek {pk}");
                    while ctx.tape.get_next_func::<Value>().call(ctx).truthy() {
                        loop_body!(ctx, size);
                        // println!("Condition true {}", X.load(Ordering::Relaxed));
                        X.fetch_add(1, Ordering::Relaxed);

                        // println!("Test!");
                        // The body of the while loop must not return
                        // any value
                        ctx.tape.get_next_func::<Value>().call(ctx);
                        ctx.tape.restore(tape_ptr);
                    }

                    ctx.tape.skip(size as usize);

                    None
                }

                self.push(Hint::While as u64);
                self.push(unsafe { transmute(Operation(l) as Operation<Option<Value>>) });
                let size_idx = self.future_tape.len();
                self.push(0);

                self.compile_expr(*cond);
                let body_idx = self.future_tape.len();
                self.compile_expr(*body);

                self.future_tape[size_idx] = (self.future_tape.len() - body_idx) as u64;
            }

            Expr::BinaryOp(lhs, op, rhs) => match op {
                Operator::Add => impl_op!(add, self, lhs, +, rhs),
                Operator::Sub => impl_op!(sub, self, lhs, -, rhs),
                Operator::Mul => impl_op!(mul, self, lhs, *, rhs),
                Operator::Div => impl_op!(div, self, lhs, /, rhs),
                Operator::Rem => impl_op!(rem, self, lhs, %, rhs),
                Operator::Eq => impl_op!(eq, self, lhs, ==, rhs),
                Operator::Neq => impl_op!(neq, self, lhs, !=, rhs),
                Operator::Gt => impl_op!(gt, self, lhs, >, rhs),
                Operator::Gte => impl_op!(gte, self, lhs, >=, rhs),
                Operator::Lt => impl_op!(lt, self, lhs, <=, rhs),
                Operator::Lte => impl_op!(lte, self, lhs, <, rhs),
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

    pub fn execute(&mut self) -> Value {
        unsafe { self.tape.get_next_func::<Value>().call(self) }
    }
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

    println!("Compiler: {compiler:?}");

    let mut context = CallContext::new(
        compiler.future_tape.as_ptr(),
        compiler.future_tape.len(),
        compiler.globals.len(),
    );
    context.execute();

    let x = X.load(Ordering::Relaxed);
    println!("X: {x}");

    //println!("Context: {:?}", context);
}
