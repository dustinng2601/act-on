//! GitHub Actions expression language.
//!
//! Pipeline: `${{ ... }}` -> [`lexer::Lexer`] -> [`parser::Parser`] -> [`ast::Expr`]
//! -> [`eval::Evaluator`] with [`eval::Env`].

pub mod lexer;
pub mod ast;
pub mod parser;
pub mod eval;
pub mod funcs;
pub mod interpolate;

pub use ast::Expr;
pub use eval::{Env, Evaluator, Value, DefaultStatusCheck, eval_if};
pub use interpolate::interpolate;
