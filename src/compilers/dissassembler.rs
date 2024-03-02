pub use crate::*;

pub struct Dissassembler {
    offset: u64,
    tape: Vec<u64>,
    globals: Vec<String>,
}

impl From<ImCompiler> for Dissassembler {
    fn from(value: ImCompiler) -> Self {
        Self {
            offset: 0,
            tape: value.future_tape,
            globals: value.globals,
        }
    }
}

macro_rules! impl_op_diss {
    ($self:ident, $as_fn:ident, $f:path, $op:tt) => {
        if $as_fn == $f {
            $self.dissassemble();
            eprint!(" {} ", stringify!($op));
            $self.dissassemble();
            return true;
        }
    };
}

impl Dissassembler {
    fn read(&mut self) -> u64 {
        let value = self.tape[self.offset as usize];
        self.offset += 1;

        value
    }

    fn peek(&mut self) -> u64 {
        self.tape[self.offset as usize + 1]
    }

    pub fn dissassemble_program(&mut self) {
        self.dissassemble();
        eprintln!("");
    }

    pub fn dissassemble(&mut self) {
        let element = self.read();

        if self.handle_fn(element) {
        } else if self.handle_literal(element) {
        } else if self.handle_operation(element) {
        }
    }

    fn handle_literal(&mut self, element: u64) -> bool {
        let as_fn: unsafe fn(&mut CallContext) -> Value = unsafe { transmute(element) };

        if as_fn == literals::float {
            let value = self.read();
            let value: f64 = unsafe { transmute(value) };

            eprint!("{}f64", value);
        } else {
            return false;
        }

        true
    }

    fn handle_operation(&mut self, element: u64) -> bool {
        let as_fn: unsafe fn(&mut CallContext) -> Value = unsafe { transmute(element) };

        if as_fn == operations::assign {
            eprint!("global ");

            let idx = self.read() as usize;
            eprint!("{}", self.globals[idx]);

            eprint!(" = ");
            self.dissassemble();
        } else if as_fn == operations::var {
            let idx = self.read() as usize;
            eprint!("{}", self.globals[idx]);
            return true;
        } 
        
        impl_op_diss!(self, as_fn, operations::native_op_add, +); 
        impl_op_diss!(self, as_fn, operations::native_op_sub, -); 
        impl_op_diss!(self, as_fn, operations::native_op_mul, *); 
        impl_op_diss!(self, as_fn, operations::native_op_div, /); 
        impl_op_diss!(self, as_fn, operations::native_op_rem, %); 
        impl_op_diss!(self, as_fn, operations::native_op_eq, ==); 
        impl_op_diss!(self, as_fn, operations::native_op_eq, !=); 
        impl_op_diss!(self, as_fn, operations::native_op_gt, >); 
        impl_op_diss!(self, as_fn, operations::native_op_gte, >=); 
        impl_op_diss!(self, as_fn, operations::native_op_lt, <); 
        impl_op_diss!(self, as_fn, operations::native_op_lte, <=); 

        true
    }

    fn handle_fn(&mut self, element: u64) -> bool {
        let as_fn: unsafe fn(&mut CallContext) -> Value = unsafe { transmute(element) };
        let as_opt_fn: unsafe fn(&mut CallContext) -> Option<Value> = unsafe { transmute(element) };

        if as_fn == flow::block {
            eprint!("{{");
            eprint!("}}");
        } else if as_fn == flow::block_checked {
            eprint!("{{\n");
            let next_instr_idx = self.read();

            while self.offset != next_instr_idx {
                let pk = self.peek();

                match pk {
                    1001 => {
                        eprint!("return ");
                        self.offset += 1;
                        self.dissassemble();
                    }

                    1003 => {
                        self.offset += 1;
                        self.dissassemble();
                    }

                    _ => {
                        self.dissassemble();
                    }
                }

                eprint!("\n");
            }
            eprint!("}}");
        } else if as_opt_fn == flow::while_loop {
            eprint!("while ");
            let _ = self.read();
            self.dissassemble();
            eprint!(" {{\n");
            self.dissassemble();
            eprint!("\n}}");
        } else {
            return false;
        }

        true
    }
}
