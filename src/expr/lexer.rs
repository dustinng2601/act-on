//! Lexer for the GitHub Actions expression language.

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Null,
    True,
    False,
    Infinity,
    Nan,
    Int(i64),
    Float(f64),
    Str(String),
    Ident(String),
    LParen,
    RParen,
    LBracket,
    RBracket,
    StarStar, // **
    Star,     // *
    Comma,    // ,
    Dot,      // .
    Not,      // !
    Eq,       // ==
    Ne,       // !=
    Lt,       // <
    Le,       // <=
    Gt,       // >
    Ge,       // >=
    And,      // &&
    Or,       // ||
}

#[derive(Debug, Clone)]
pub struct Lexer<'a> {
    src: &'a [u8],
    pos: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(src: &'a str) -> Self {
        Self {
            src: src.as_bytes(),
            pos: 0,
        }
    }

    fn peek(&self) -> Option<u8> {
        self.src.get(self.pos).copied()
    }

    fn peek2(&self) -> Option<u8> {
        self.src.get(self.pos + 1).copied()
    }

    fn bump(&mut self) -> Option<u8> {
        let b = self.peek()?;
        self.pos += 1;
        Some(b)
    }

    fn skip_ws(&mut self) {
        while let Some(b) = self.peek() {
            if b.is_ascii_whitespace() {
                self.pos += 1;
            } else {
                break;
            }
        }
    }

    pub fn next(&mut self) -> Option<Token> {
        self.skip_ws();
        let b = self.bump()?;
        match b {
            b'(' => Some(Token::LParen),
            b')' => Some(Token::RParen),
            b'[' => Some(Token::LBracket),
            b']' => Some(Token::RBracket),
            b',' => Some(Token::Comma),
            b'.' => Some(Token::Dot),
            b'*' => {
                if self.peek() == Some(b'*') {
                    self.pos += 1;
                    Some(Token::StarStar)
                } else {
                    Some(Token::Star)
                }
            }
            b'!' => Some(Token::Not),
            b'=' => {
                if self.peek() == Some(b'=') {
                    self.pos += 1;
                    Some(Token::Eq)
                } else {
                    None
                }
            }
            b'<' => {
                if self.peek() == Some(b'=') {
                    self.pos += 1;
                    Some(Token::Le)
                } else {
                    Some(Token::Lt)
                }
            }
            b'>' => {
                if self.peek() == Some(b'=') {
                    self.pos += 1;
                    Some(Token::Ge)
                } else {
                    Some(Token::Gt)
                }
            }
            b'&' if self.peek() == Some(b'&') => {
                self.pos += 1;
                Some(Token::And)
            }
            b'|' if self.peek() == Some(b'|') => {
                self.pos += 1;
                Some(Token::Or)
            }
            b'\'' => self.lex_str(b'\''),
            b'"' => self.lex_str(b'"'),
            c if c.is_ascii_digit() => self.lex_number(c),
            c if c.is_ascii_alphabetic() || c == b'_' => self.lex_ident(c),
            _ => None,
        }
    }

    fn lex_str(&mut self, quote: u8) -> Option<Token> {
        let mut s = String::new();
        while let Some(b) = self.peek() {
            if b == quote {
                self.pos += 1;
                return Some(Token::Str(s));
            }
            self.pos += 1;
            if b == b'\\' {
                let esc = self.bump()?;
                s.push(match esc {
                    b'n' => '\n',
                    b't' => '\t',
                    b'r' => '\r',
                    b'\\' => '\\',
                    other => other as char,
                });
            } else {
                s.push(b as char);
            }
        }
        None
    }

    fn lex_number(&mut self, _first: u8) -> Option<Token> {
        let start = self.pos - 1;
        let mut is_float = false;
        while let Some(b) = self.peek() {
            if b.is_ascii_digit() {
                self.pos += 1;
            } else if b == b'.' {
                is_float = true;
                self.pos += 1;
            } else {
                break;
            }
        }
        let raw = std::str::from_utf8(&self.src[start..self.pos]).ok()?;
        if is_float {
            raw.parse::<f64>().ok().map(Token::Float)
        } else {
            raw.parse::<i64>().ok().map(Token::Int)
        }
    }

    fn lex_ident(&mut self, first: u8) -> Option<Token> {
        let mut s = String::new();
        s.push(first as char);
        while let Some(b) = self.peek() {
            // No '.' here. A dot is the property operator, and the parser
            // already builds an attribute access from `Token::Dot` — but it
            // never saw one, because this loop absorbed the dot and handed back
            // a single identifier like `env.NAME`, which resolves to nothing.
            if b.is_ascii_alphanumeric() || b == b'_' || b == b'-' {
                s.push(b as char);
                self.pos += 1;
            } else {
                break;
            }
        }
        match s.as_str() {
            "true" => Some(Token::True),
            "false" => Some(Token::False),
            "null" => Some(Token::Null),
            "Infinity" => Some(Token::Infinity),
            "NaN" => Some(Token::Nan),
            _ => Some(Token::Ident(s)),
        }
    }
}
