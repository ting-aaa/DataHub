use std::collections::BTreeMap;

use datahub_kernel::SchemaDefinition;

use crate::{BinaryOp, FormulaError, FormulaExpr, FormulaValue, UnaryOp};

#[derive(Debug, Clone, PartialEq)]
enum TokenKind {
    Number(f64),
    String(String),
    Identifier(String),
    LeftParen,
    RightParen,
    Comma,
    Plus,
    Minus,
    Star,
    Slash,
    Bang,
    EqualEqual,
    BangEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    AndAnd,
    OrOr,
    End,
}

#[derive(Debug, Clone)]
struct Token {
    kind: TokenKind,
    position: usize,
}

struct Lexer<'a> {
    source: &'a [u8],
    position: usize,
}

impl<'a> Lexer<'a> {
    const fn new(source: &'a str) -> Self {
        Self {
            source: source.as_bytes(),
            position: 0,
        }
    }

    fn tokenize(mut self) -> Result<Vec<Token>, FormulaError> {
        let mut tokens = Vec::new();
        loop {
            let token = self.next_token()?;
            let is_end = token.kind == TokenKind::End;
            tokens.push(token);
            if is_end {
                return Ok(tokens);
            }
        }
    }

    fn next_token(&mut self) -> Result<Token, FormulaError> {
        while self.peek().is_some_and(|byte| byte.is_ascii_whitespace()) {
            self.position += 1;
        }
        let position = self.position;
        let Some(byte) = self.advance() else {
            return Ok(Token {
                kind: TokenKind::End,
                position,
            });
        };
        let kind = match byte {
            b'(' => TokenKind::LeftParen,
            b')' => TokenKind::RightParen,
            b',' => TokenKind::Comma,
            b'+' => TokenKind::Plus,
            b'-' => TokenKind::Minus,
            b'*' => TokenKind::Star,
            b'/' => TokenKind::Slash,
            b'!' if self.take(b'=') => TokenKind::BangEqual,
            b'!' => TokenKind::Bang,
            b'=' if self.take(b'=') => TokenKind::EqualEqual,
            b'<' if self.take(b'=') => TokenKind::LessEqual,
            b'<' => TokenKind::Less,
            b'>' if self.take(b'=') => TokenKind::GreaterEqual,
            b'>' => TokenKind::Greater,
            b'&' if self.take(b'&') => TokenKind::AndAnd,
            b'|' if self.take(b'|') => TokenKind::OrOr,
            b'"' => TokenKind::String(self.string(position)?),
            value if value.is_ascii_digit() || value == b'.' => {
                TokenKind::Number(self.number(position)?)
            }
            value if value.is_ascii_alphabetic() || value == b'_' => {
                TokenKind::Identifier(self.identifier())
            }
            _ => return Err(parse_error(position, "unexpected character")),
        };
        Ok(Token { kind, position })
    }

    fn string(&mut self, start: usize) -> Result<String, FormulaError> {
        let mut value = String::new();
        while let Some(byte) = self.advance() {
            match byte {
                b'"' => return Ok(value),
                b'\\' => match self.advance() {
                    Some(b'n') => value.push('\n'),
                    Some(b'r') => value.push('\r'),
                    Some(b't') => value.push('\t'),
                    Some(b'"') => value.push('"'),
                    Some(b'\\') => value.push('\\'),
                    Some(_) => return Err(parse_error(self.position - 1, "unsupported escape")),
                    None => return Err(parse_error(start, "unterminated string")),
                },
                value_byte if value_byte.is_ascii() => value.push(char::from(value_byte)),
                _ => {
                    return Err(parse_error(
                        self.position - 1,
                        "formula strings must be UTF-8 ASCII",
                    ));
                }
            }
        }
        Err(parse_error(start, "unterminated string"))
    }

    fn number(&mut self, start: usize) -> Result<f64, FormulaError> {
        while self
            .peek()
            .is_some_and(|byte| byte.is_ascii_digit() || byte == b'.')
        {
            self.position += 1;
        }
        if self.peek().is_some_and(|byte| byte == b'e' || byte == b'E') {
            self.position += 1;
            if self.peek().is_some_and(|byte| byte == b'+' || byte == b'-') {
                self.position += 1;
            }
            while self.peek().is_some_and(|byte| byte.is_ascii_digit()) {
                self.position += 1;
            }
        }
        let raw = std::str::from_utf8(&self.source[start..self.position])
            .map_err(|_| parse_error(start, "invalid number"))?;
        let value = raw
            .parse::<f64>()
            .map_err(|_| parse_error(start, "invalid number"))?;
        if value.is_finite() {
            Ok(value)
        } else {
            Err(parse_error(start, "number must be finite"))
        }
    }

    fn identifier(&mut self) -> String {
        let start = self.position - 1;
        while self
            .peek()
            .is_some_and(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        {
            self.position += 1;
        }
        String::from_utf8_lossy(&self.source[start..self.position]).into_owned()
    }

    fn peek(&self) -> Option<u8> {
        self.source.get(self.position).copied()
    }

    fn advance(&mut self) -> Option<u8> {
        let value = self.peek()?;
        self.position += 1;
        Some(value)
    }

    fn take(&mut self, expected: u8) -> bool {
        if self.peek() == Some(expected) {
            self.position += 1;
            true
        } else {
            false
        }
    }
}

struct Parser<'a> {
    tokens: Vec<Token>,
    current: usize,
    fields: BTreeMap<&'a str, datahub_kernel::FieldId>,
}

impl<'a> Parser<'a> {
    fn new(tokens: Vec<Token>, schema: &'a SchemaDefinition) -> Self {
        let fields = schema
            .fields
            .iter()
            .map(|field| (field.name.as_str(), field.id))
            .collect();
        Self {
            tokens,
            current: 0,
            fields,
        }
    }

    fn parse(mut self) -> Result<FormulaExpr, FormulaError> {
        let expression = self.or()?;
        if self.peek().kind != TokenKind::End {
            return Err(parse_error(
                self.peek().position,
                "unexpected trailing token",
            ));
        }
        Ok(expression)
    }

    fn or(&mut self) -> Result<FormulaExpr, FormulaError> {
        let mut expression = self.and()?;
        while self.matches(&TokenKind::OrOr) {
            expression = binary(BinaryOp::Or, expression, self.and()?);
        }
        Ok(expression)
    }

    fn and(&mut self) -> Result<FormulaExpr, FormulaError> {
        let mut expression = self.equality()?;
        while self.matches(&TokenKind::AndAnd) {
            expression = binary(BinaryOp::And, expression, self.equality()?);
        }
        Ok(expression)
    }

    fn equality(&mut self) -> Result<FormulaExpr, FormulaError> {
        let mut expression = self.comparison()?;
        loop {
            let op = if self.matches(&TokenKind::EqualEqual) {
                Some(BinaryOp::Equal)
            } else if self.matches(&TokenKind::BangEqual) {
                Some(BinaryOp::NotEqual)
            } else {
                None
            };
            let Some(op) = op else { return Ok(expression) };
            expression = binary(op, expression, self.comparison()?);
        }
    }

    fn comparison(&mut self) -> Result<FormulaExpr, FormulaError> {
        let mut expression = self.term()?;
        loop {
            let op = if self.matches(&TokenKind::Less) {
                Some(BinaryOp::Less)
            } else if self.matches(&TokenKind::LessEqual) {
                Some(BinaryOp::LessEqual)
            } else if self.matches(&TokenKind::Greater) {
                Some(BinaryOp::Greater)
            } else if self.matches(&TokenKind::GreaterEqual) {
                Some(BinaryOp::GreaterEqual)
            } else {
                None
            };
            let Some(op) = op else { return Ok(expression) };
            expression = binary(op, expression, self.term()?);
        }
    }

    fn term(&mut self) -> Result<FormulaExpr, FormulaError> {
        let mut expression = self.factor()?;
        loop {
            let op = if self.matches(&TokenKind::Plus) {
                Some(BinaryOp::Add)
            } else if self.matches(&TokenKind::Minus) {
                Some(BinaryOp::Subtract)
            } else {
                None
            };
            let Some(op) = op else { return Ok(expression) };
            expression = binary(op, expression, self.factor()?);
        }
    }

    fn factor(&mut self) -> Result<FormulaExpr, FormulaError> {
        let mut expression = self.unary()?;
        loop {
            let op = if self.matches(&TokenKind::Star) {
                Some(BinaryOp::Multiply)
            } else if self.matches(&TokenKind::Slash) {
                Some(BinaryOp::Divide)
            } else {
                None
            };
            let Some(op) = op else { return Ok(expression) };
            expression = binary(op, expression, self.unary()?);
        }
    }

    fn unary(&mut self) -> Result<FormulaExpr, FormulaError> {
        if self.matches(&TokenKind::Bang) {
            return Ok(FormulaExpr::Unary {
                op: UnaryOp::Not,
                expression: Box::new(self.unary()?),
            });
        }
        if self.matches(&TokenKind::Minus) {
            return Ok(FormulaExpr::Unary {
                op: UnaryOp::Negate,
                expression: Box::new(self.unary()?),
            });
        }
        self.primary()
    }

    fn primary(&mut self) -> Result<FormulaExpr, FormulaError> {
        let token = self.advance().clone();
        match token.kind {
            TokenKind::Number(value) => Ok(literal(FormulaValue::Number(value))),
            TokenKind::String(value) => Ok(literal(FormulaValue::String(value))),
            TokenKind::Identifier(identifier) if identifier == "true" => {
                Ok(literal(FormulaValue::Bool(true)))
            }
            TokenKind::Identifier(identifier) if identifier == "false" => {
                Ok(literal(FormulaValue::Bool(false)))
            }
            TokenKind::Identifier(identifier) if identifier == "null" => {
                Ok(literal(FormulaValue::Null))
            }
            TokenKind::Identifier(identifier) if identifier == "if" => {
                self.if_expression(token.position)
            }
            TokenKind::Identifier(identifier) => {
                let field_id = self
                    .fields
                    .get(identifier.as_str())
                    .copied()
                    .ok_or(FormulaError::UnknownField(identifier))?;
                Ok(FormulaExpr::Field { field_id })
            }
            TokenKind::LeftParen => {
                let expression = self.or()?;
                self.consume(&TokenKind::RightParen, "expected `)`")?;
                Ok(expression)
            }
            _ => Err(parse_error(token.position, "expected expression")),
        }
    }

    fn if_expression(&mut self, position: usize) -> Result<FormulaExpr, FormulaError> {
        self.consume(&TokenKind::LeftParen, "expected `(` after `if`")?;
        let condition = self.or()?;
        self.consume(&TokenKind::Comma, "expected `,` after if condition")?;
        let then_expression = self.or()?;
        self.consume(&TokenKind::Comma, "expected `,` after if result")?;
        let else_expression = self.or()?;
        self.consume(&TokenKind::RightParen, "expected `)` after if expression")
            .map_err(|_| parse_error(position, "incomplete if expression"))?;
        Ok(FormulaExpr::If {
            condition: Box::new(condition),
            then_expression: Box::new(then_expression),
            else_expression: Box::new(else_expression),
        })
    }

    fn matches(&mut self, expected: &TokenKind) -> bool {
        if &self.peek().kind == expected {
            self.current += 1;
            true
        } else {
            false
        }
    }

    fn consume(&mut self, expected: &TokenKind, message: &str) -> Result<(), FormulaError> {
        if self.matches(expected) {
            Ok(())
        } else {
            Err(parse_error(self.peek().position, message))
        }
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.current]
    }

    fn advance(&mut self) -> &Token {
        let index = self.current;
        if self.tokens[index].kind != TokenKind::End {
            self.current += 1;
        }
        &self.tokens[index]
    }
}

/// Parses a formula and resolves every field name to its stable `FieldId`.
///
/// # Errors
/// Returns a position-bearing parse error or [`FormulaError::UnknownField`].
pub fn parse_formula(source: &str, schema: &SchemaDefinition) -> Result<FormulaExpr, FormulaError> {
    Parser::new(Lexer::new(source).tokenize()?, schema).parse()
}

fn binary(op: BinaryOp, left: FormulaExpr, right: FormulaExpr) -> FormulaExpr {
    FormulaExpr::Binary {
        op,
        left: Box::new(left),
        right: Box::new(right),
    }
}

fn literal(value: FormulaValue) -> FormulaExpr {
    FormulaExpr::Literal { value }
}

fn parse_error(position: usize, message: impl Into<String>) -> FormulaError {
    FormulaError::Parse {
        position,
        message: message.into(),
    }
}
