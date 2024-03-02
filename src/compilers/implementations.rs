pub mod literals {
    use crate::*;

    pub unsafe fn tr(_: &mut CallContext) -> Value {
        Value::Boolean(true)
    }

    pub unsafe fn fl(_: &mut CallContext) -> Value {
        Value::Boolean(false)
    }

    pub unsafe fn float(ctx: &mut CallContext) -> Value {
        //println!("float => _____");
        Value::Float(transmute(ctx.tape.get_next()))
    }
}

pub mod flow {
    use crate::*;

    macro_rules! loop_body {
        ($ctx:ident, $next_instr:ident) => {
            if 1001 <= $ctx.tape.read() && $ctx.tape.read() <= 2000 as u64 {
                let v = $ctx.tape.read();
                $ctx.tape.skip(1);

                match v {
                    1001 => {
                        let value = $ctx.tape.get_next_func::<Value>().call($ctx);
                        $ctx.tape.move_to($next_instr as usize);
                        return value.into();
                    }
                    1003 => {
                        let value = $ctx.tape.get_next_func::<Option<Value>>().call($ctx);
                        if let Some(x) = value {
                            $ctx.tape.move_to($next_instr as usize);
                            return x.into();
                        }
                        continue;
                    }
                    x => panic!("Invalid block hint: {x}"),
                }
            }
        };
    }
    pub unsafe fn block_checked(ctx: &mut CallContext) -> Value {
        let next_instr = ctx.tape.get_next() as usize;

        while ctx.tape.offset < next_instr {
            loop_body!(ctx, next_instr);

            let func = ctx.tape.get_next_func::<Value>();
            func.call(ctx);
        }

        Value::Nil
    }

    pub unsafe fn block(ctx: &mut CallContext) -> Value {
        let next_instr = ctx.tape.get_next() as usize;

        while ctx.tape.offset < next_instr {
            let func = ctx.tape.get_next_func::<Value>();
            func.call(ctx);
        }
        Value::Nil
    }

    pub unsafe fn while_loop(ctx: &mut CallContext) -> Option<Value> {
        let next_idx: u64 = ctx.tape.get_next();
        let tape_ptr = ctx.tape.save();

        while ctx.tape.get_next_func::<Value>().call(ctx).truthy() {
            loop_body!(ctx, next_idx);

            ctx.tape.get_next_func::<Value>().call(ctx);
            ctx.tape.restore(tape_ptr);
        }

        ctx.tape.skip(next_idx as usize);

        None
    }
}

pub mod operations {
    use crate::*;

    macro_rules! impl_op {
        ($name:ident, $op:tt) => {
            pub unsafe fn $name(ctx: &mut CallContext) -> Value {
                let lhs = ctx.tape.get_next_func::<Value>().call(ctx);
                let rhs = ctx.tape.get_next_func::<Value>().call(ctx);

                impl_apply_op!(lhs, $op, rhs)
            }
        };
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

    impl_op!(native_op_add, +);
    impl_op!(native_op_sub, -);
    impl_op!(native_op_mul, *);
    impl_op!(native_op_div, /);
    impl_op!(native_op_rem, %);
    impl_op!(native_op_eq, ==);
    impl_op!(native_op_neq, !=);
    impl_op!(native_op_gt, >);
    impl_op!(native_op_gte, >=);
    impl_op!(native_op_lt, <);
    impl_op!(native_op_lte, <=);
}
