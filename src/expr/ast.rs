//! AST for the GitHub Actions expression language.

use crate::expr::lexer::Token;

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
    /// Identifier (context name or property, e.g. `github`, `env`, `FOO`).
    Ident(String),
    /// `expr[index]`
    Index(Box<Expr>, Box<Expr>),
    /// `expr.property` — `property` is a literal attribute name.
    Attr(Box<Expr>, String),
    /// `expr.*` — array deref
    ArrayDeref(Box<Expr>),
    /// `!expr`
    Not(Box<Expr>),
    /// `a op b`
    Compare(CompareOp, Box<Expr>, Box<Expr>),
    And(Box<Expr>, Box<Expr>),
    Or(Box<Expr>, Box<Expr>),
    /// `name(arg1, arg2, ...)`
    Call(String, Vec<Expr>),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CompareOp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

/// Convert a single token to an atomic literal Expr (used inside the parser).
pub fn literal_of(tok: Token) -> Option<Expr> {
    Some(match tok {
        Token::Null => Expr::Null,
        Token::True => Expr::Bool(true),
        Token::False => Expr::Bool(false),
        Token::Infinity => Expr::Float(f64::INFINITY),
        Token::Nan => Expr::Float(f64::NAN),
        Token::Int(i) => Expr::Int(i),
        Token::Float(f) => Expr::Float(f),
        Token::Str(s) => Expr::Str(s),
        _ => return None,
    })
}
