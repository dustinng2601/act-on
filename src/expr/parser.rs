//! Pratt parser for the GitHub Actions expression language.
//!
//! Grammar (subset, good enough for the v1 milestone):
//!
//! ```text
//! expr      := or
//! or        := and ( "||" and )*
//! and       := cmp ( "&&" cmp )*
//! cmp       := unary ( ("=="|"!="|"<"|"<="|">"|">=") unary )?
//! unary     := "!"? postfix
//! postfix   := primary ( "." ident | "[" expr "]" | ".*" )*
//! primary   := null | true | false | number | string | ident | ident "(" args ")"
//! args      := expr ( "," expr )*
//! ```

use super::ast::{CompareOp, Expr};
use super::lexer::{Lexer, Token};

pub struct Parser<'a> {
    lexer: Lexer<'a>,
    peek: Option<Token>,
}

impl<'a> Parser<'a> {
    pub fn new(src: &'a str) -> Self {
        let mut lexer = Lexer::new(src);
        let peek = lexer.next();
        Self { lexer, peek }
    }

    fn bump(&mut self) -> Option<Token> {
        let cur = self.peek.take();
        self.peek = self.lexer.next();
        cur
    }

    fn peek_is(&self, t: &Token) -> bool {
        self.peek.as_ref() == Some(t)
    }

    pub fn parse(&mut self) -> anyhow::Result<Expr> {
        let e = self.parse_or()?;
        if self.peek.is_some() {
            anyhow::bail!("unexpected trailing token: {:?}", self.peek);
        }
        Ok(e)
    }

    fn parse_or(&mut self) -> anyhow::Result<Expr> {
        let mut lhs = self.parse_and()?;
        while self.peek_is(&Token::Or) {
            self.bump();
            let rhs = self.parse_and()?;
            lhs = Expr::Or(Box::new(lhs), Box::new(rhs));
        }
        Ok(lhs)
    }

    fn parse_and(&mut self) -> anyhow::Result<Expr> {
        let mut lhs = self.parse_cmp()?;
        while self.peek_is(&Token::And) {
            self.bump();
            let rhs = self.parse_cmp()?;
            lhs = Expr::And(Box::new(lhs), Box::new(rhs));
        }
        Ok(lhs)
    }

    fn parse_cmp(&mut self) -> anyhow::Result<Expr> {
        let lhs = self.parse_unary()?;
        let op = match self.peek {
            Some(Token::Eq) => Some(CompareOp::Eq),
            Some(Token::Ne) => Some(CompareOp::Ne),
            Some(Token::Lt) => Some(CompareOp::Lt),
            Some(Token::Le) => Some(CompareOp::Le),
            Some(Token::Gt) => Some(CompareOp::Gt),
            Some(Token::Ge) => Some(CompareOp::Ge),
            _ => None,
        };
        if let Some(op) = op {
            self.bump();
            let rhs = self.parse_unary()?;
            return Ok(Expr::Compare(op, Box::new(lhs), Box::new(rhs)));
        }
        Ok(lhs)
    }

    fn parse_unary(&mut self) -> anyhow::Result<Expr> {
        if self.peek_is(&Token::Not) {
            self.bump();
            let inner = self.parse_unary()?;
            return Ok(Expr::Not(Box::new(inner)));
        }
        self.parse_postfix()
    }

    fn parse_postfix(&mut self) -> anyhow::Result<Expr> {
        let mut e = self.parse_primary()?;
        loop {
            match &self.peek {
                Some(Token::Dot) => {
                    self.bump();
                    if self.peek_is(&Token::Star) {
                        self.bump();
                        e = Expr::ArrayDeref(Box::new(e));
                    } else if let Some(Token::Ident(name)) = self.peek.clone() {
                        self.bump();
                        e = Expr::Attr(Box::new(e), name);
                    } else {
                        anyhow::bail!("expected identifier or '*' after '.'");
                    }
                }
                Some(Token::LBracket) => {
                    self.bump();
                    let inner = self.parse_or()?;
                    if !self.peek_is(&Token::RBracket) {
                        anyhow::bail!("expected ']' in index expression");
                    }
                    self.bump();
                    e = Expr::Index(Box::new(e), Box::new(inner));
                }
                _ => break,
            }
        }
        Ok(e)
    }

    fn parse_primary(&mut self) -> anyhow::Result<Expr> {
        let tok = self
            .bump()
            .ok_or_else(|| anyhow::anyhow!("unexpected end of expression"))?;
        // Literals first - they don't need lookahead.
        match &tok {
            Token::Null
            | Token::True
            | Token::False
            | Token::Infinity
            | Token::Nan
            | Token::Int(_)
            | Token::Float(_)
            | Token::Str(_) => {
                return Ok(
                    super::ast::literal_of(tok).expect("literal_of covers all literal variants")
                );
            }
            _ => {}
        }
        match tok {
            Token::LParen => {
                let e = self.parse_or()?;
                if !self.peek_is(&Token::RParen) {
                    anyhow::bail!("expected ')'");
                }
                self.bump();
                Ok(e)
            }
            Token::Ident(name) => {
                if self.peek_is(&Token::LParen) {
                    self.bump();
                    let mut args = Vec::new();
                    if !self.peek_is(&Token::RParen) {
                        loop {
                            args.push(self.parse_or()?);
                            if self.peek_is(&Token::Comma) {
                                self.bump();
                            } else {
                                break;
                            }
                        }
                    }
                    if !self.peek_is(&Token::RParen) {
                        anyhow::bail!("expected ')'");
                    }
                    self.bump();
                    Ok(Expr::Call(name, args))
                } else {
                    Ok(Expr::Ident(name))
                }
            }
            other => anyhow::bail!("unexpected token: {other:?}"),
        }
    }
}

/// Convenience: parse a single expression string.
pub fn parse(src: &str) -> anyhow::Result<Expr> {
    Parser::new(src).parse()
}
