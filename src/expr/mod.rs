//! GitHub Actions expression language.
//!
//! Pipeline: `${{ ... }}` -> [`lexer::Lexer`] -> [`parser::Parser`] -> [`ast::Expr`]
//! -> [`eval::Evaluator`] with [`eval::Env`].

pub mod ast;
pub mod eval;
pub mod funcs;
pub mod interpolate;
pub mod lexer;
pub mod parser;

pub use ast::Expr;
pub use eval::{eval_if, DefaultStatusCheck, Env, Evaluator, Value};
pub use interpolate::interpolate;
