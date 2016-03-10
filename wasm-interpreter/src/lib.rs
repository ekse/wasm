extern crate byteorder;
extern crate wasm_ast;

use byteorder::{ByteOrder, LittleEndian};

use std::iter::repeat;
use std::default::Default;

use wasm_ast::{BinOp, Expr, UnaryOp};
use wasm_ast::BinOp::{Add, And, DivS, DivU, Eq, GeS, GeU, GtS, GtU, LeS, LeU, LtS, LtU};
use wasm_ast::BinOp::{Mul, Ne, Or, RemS, RemU, RotL, RotR, Shl, ShrS, ShrU, Sub, Xor};
use wasm_ast::Const::{F32Const, F64Const, I32Const, I64Const};
use wasm_ast::Expr::{BinOpExpr, ConstExpr, GetLocalExpr, GrowMemoryExpr, IfThenExpr, IfThenElseExpr, LoadExpr, NopExpr, SetLocalExpr, StoreExpr, UnaryOpExpr};
use wasm_ast::Typ::{F32, F64, I32, I64};
use wasm_ast::UnaryOp::{Clz, Ctz, Popcnt, Eqz};

trait Interpreter<T> {

    fn interpret_binop(&self, op: &BinOp, lhs: T, rhs: T) -> T;

    fn interpret_unop(&self, op: &UnaryOp, arg: T) -> T;

    fn from_f32(&self, _: f32) -> T {
        panic!("Type error.")
    }

    fn from_f64(&self, _: f64) -> T {
        panic!("Type error.")
    }

    fn from_i32(&self, _: u32) -> T {
        panic!("Type error.")
    }

    fn from_i64(&self, _: u64) -> T {
        panic!("Type error.")
    }

    fn from_raw(&self, _: u64) -> T;

    fn to_raw(&self, _: T) -> u64;

    fn interpret_expr(&mut self, expr: &Expr, locals: &mut[u64], heap: &mut Vec<u8>) -> T
        where Self: Interpreter<f32> + Interpreter<f64> + Interpreter<u32> + Interpreter<u64>,
              T: Copy + Default,
    {
        // NOTE: currently only handling the control flow that can be dealt with in direct style.
        // More sophisticated control flow will require a technique for handling a CFG,
        // e.g. functional SSA.
        match expr {
            &BinOpExpr(_, ref op, ref lhs, ref rhs) => {
                let lhs = self.interpret_expr(lhs, locals, heap);
                let rhs = self.interpret_expr(rhs, locals, heap);
                self.interpret_binop(op, lhs, rhs)
            },
            &ConstExpr(F32Const(value)) => self.from_f32(value),
            &ConstExpr(F64Const(value)) => self.from_f64(value),
            &ConstExpr(I32Const(value)) => self.from_i32(value),
            &ConstExpr(I64Const(value)) => self.from_i64(value),
            &GetLocalExpr(ref var) => self.from_raw(locals[var.position]),
            &GrowMemoryExpr(ref ext) => {
                let result: u32 = heap.len() as u32;
                let ext: u32 = self.interpret_expr(ext, locals, heap);
                heap.extend(repeat(0).take(ext as usize));
                self.from_i32(result)
            },
            &IfThenExpr(ref cond, ref true_branch) => {
                let cond: u32 = self.interpret_expr(cond, locals, heap);
                if cond == 0 {
                    T::default()
                } else {
                    self.interpret_expr(true_branch, locals, heap)
                }
            },
            &IfThenElseExpr(ref cond, ref true_branch, ref false_branch) => {
                let cond: u32 = self.interpret_expr(cond, locals, heap);
                if cond == 0 {
                    self.interpret_expr(false_branch, locals, heap)
                } else {
                    self.interpret_expr(true_branch, locals, heap)
                }
            },
            &LoadExpr(F32, ref addr) => {
                let addr: u32 = self.interpret_expr(addr, locals, heap);
                let value: f32 = LittleEndian::read_f32(&heap[addr as usize..]);
                self.from_f32(value)
            },
            &LoadExpr(F64, ref addr) => {
                let addr: u32 = self.interpret_expr(addr, locals, heap);
                let value: f64 = LittleEndian::read_f64(&heap[addr as usize..]);
                self.from_f64(value)
            },
            &LoadExpr(I32, ref addr) => {
                let addr: u32 = self.interpret_expr(addr, locals, heap);
                let value: u32 = LittleEndian::read_u32(&heap[addr as usize..]);
                self.from_i32(value)
            },
            &LoadExpr(I64, ref addr) => {
                let addr: u32 = self.interpret_expr(addr, locals, heap);
                let value: u64 = LittleEndian::read_u64(&heap[addr as usize..]);
                self.from_i64(value)
            },
            &NopExpr => T::default(),
            &SetLocalExpr(ref var, ref value) => {
                let value: T = self.interpret_expr(value, locals, heap);
                locals[var.position] = self.to_raw(value);
                value
            },
            &StoreExpr(F32, ref addr, ref value) => {
                let addr: u32 = self.interpret_expr(addr, locals, heap);
                let value: f32 = self.interpret_expr(value, locals, heap);
                LittleEndian::write_f32(&mut heap[addr as usize..], value);
                self.from_f32(value)
            },
            &StoreExpr(F64, ref addr, ref value) => {
                let addr: u32 = self.interpret_expr(addr, locals, heap);
                let value: f64 = self.interpret_expr(value, locals, heap);
                LittleEndian::write_f64(&mut heap[addr as usize..], value);
                self.from_f64(value)
            },
            &StoreExpr(I32, ref addr, ref value) => {
                let addr: u32 = self.interpret_expr(addr, locals, heap);
                let value: u32 = self.interpret_expr(value, locals, heap);
                LittleEndian::write_u32(&mut heap[addr as usize..], value);
                self.from_i32(value)
            },
            &StoreExpr(I64, ref addr, ref value) => {
                let addr: u32 = self.interpret_expr(addr, locals, heap);
                let value: u64 = self.interpret_expr(value, locals, heap);
                LittleEndian::write_u64(&mut heap[addr as usize..], value);
                self.from_i64(value)
            },
            &UnaryOpExpr(_, ref op, ref arg) => {
                let arg = self.interpret_expr(arg, locals, heap);
                self.interpret_unop(op, arg)
            },
       }
    }

}

pub struct Program;

impl Interpreter<u32> for Program {

    fn interpret_binop(&self, op: &BinOp, lhs: u32, rhs: u32) -> u32 {
        match op {
            &Add => (lhs.wrapping_add(rhs)),
            &And => (lhs & rhs),
            &DivU => (lhs / rhs),
            &DivS => ((lhs as i32) / (rhs as i32)) as u32,
            &Eq => (lhs == rhs) as u32,
            &GeS => ((lhs as i32) >= (rhs as i32)) as u32,
            &GeU => (lhs >= rhs) as u32,
            &GtS => ((lhs as i32) > (rhs as i32)) as u32,
            &GtU => (lhs > rhs) as u32,
            &LeS => ((lhs as i32) <= (rhs as i32)) as u32,
            &LeU => (lhs <= rhs) as u32,
            &LtS => ((lhs as i32) < (rhs as i32)) as u32,
            &LtU => (lhs < rhs) as u32,
            &Mul => (lhs.wrapping_mul(rhs)),
            &Ne => (lhs != rhs) as u32,
            &Or => (lhs | rhs),
            &RemS => ((lhs as i32) % (rhs as i32)) as u32,
            &RemU => (lhs % rhs),
            &RotL => (lhs.rotate_left(rhs)),
            &RotR => (lhs.rotate_right(rhs)),
            &Shl => (lhs.wrapping_shl(rhs)),
            &ShrS => ((lhs as i32).wrapping_shr(rhs)) as u32,
            &ShrU => (lhs.wrapping_shr(rhs)),
            &Sub => (lhs.wrapping_sub(rhs)),
            &Xor => (lhs ^ rhs),
        }
    }

    fn interpret_unop(&self, op: &UnaryOp, arg: u32) -> u32 {
        match op {
            &Clz => arg.leading_zeros(),
            &Ctz => arg.trailing_zeros(),
            &Popcnt => arg.count_ones(),
            &Eqz => (arg == 0) as u32,
        }
    }
    
    fn from_i32(&self, value: u32) -> u32 {
        value
    }

    fn from_raw(&self, value: u64) -> u32 {
        value as u32
    }

    fn to_raw(&self, value: u32) -> u64 {
        value as u64
    }

}
