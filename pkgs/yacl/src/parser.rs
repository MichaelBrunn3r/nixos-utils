use std::iter::Peekable;

use crate::ast::{AST, BinaryOp, Expr, KV, UnaryOp};
use crate::lexer::{Lexer, LexerError, Token};

//region Parser
pub struct Parser<'input> {
    tokens: Peekable<Lexer<'input>>,
}

impl<'input> Parser<'input> {
    pub fn new(input: &'input str) -> Self {
        Self {
            tokens: Lexer::new(input).peekable(),
        }
    }

    pub fn parse(mut self) -> Result<AST<'input>, ParseError> {
        let mut pairs = Vec::new();
        self.skip_separators()?;

        while self.tokens.peek().is_some() {
            pairs.push(self.parse_kv()?);
            self.skip_separators()?;
        }

        Ok(AST { pairs })
    }

    fn parse_kv(&mut self) -> Result<KV<'input>, ParseError> {
        let key = match self.next_token()? {
            Token::Id(key) => key,
            Token::Str(key) => key,
            token => Err(self.unexpected(token, "expected a key"))?,
        };

        self.expect_next_token(Token::Eq)?;
        let value = self.parse_value()?;

        Ok(KV { key, expr: value })
    }

    fn parse_value(&mut self) -> Result<Expr<'input>, ParseError> {
        self.parse_expression(0)
    }

    fn parse_expression(&mut self, min_binding_power: u8) -> Result<Expr<'input>, ParseError> {
        let mut left = self.parse_prefix()?;

        loop {
            let Some(Ok(token)) = self.tokens.peek() else {
                break;
            };
            let Some((left_binding_power, right_binding_power, operator)) =
                Self::infix_binding_power(token)
            else {
                break;
            };

            if left_binding_power < min_binding_power {
                break;
            }

            self.next_token()?;
            let right = self.parse_expression(right_binding_power)?;
            left = Expr::Binary {
                left: Box::new(left),
                op: operator,
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_prefix(&mut self) -> Result<Expr<'input>, ParseError> {
        match self.next_token()? {
            Token::Bool(value) => Ok(Expr::Bool(value)),
            Token::Int(value) => Ok(Expr::Int(value)),
            Token::Float(value) => Ok(Expr::Float(value)),
            Token::Str(value) => Ok(Expr::Str(value)),
            Token::Id(value) => {
                if matches!(self.tokens.peek(), Some(Ok(Token::LParen))) {
                    self.parse_call(value)
                } else {
                    Ok(Expr::Id(value))
                }
            }
            Token::Add => self.parse_unary(UnaryOp::Positive),
            Token::Sub => self.parse_unary(UnaryOp::Negative),
            token => Err(self.unexpected(token, "expected an expression")),
        }
    }

    fn parse_unary(&mut self, operator: UnaryOp) -> Result<Expr<'input>, ParseError> {
        let value = self.parse_expression(25)?;
        Ok(Expr::Unary {
            op: operator,
            value: Box::new(value),
        })
    }

    fn parse_call(&mut self, name: &'input str) -> Result<Expr<'input>, ParseError> {
        self.expect_next_token(Token::LParen)?;
        let mut arguments = Vec::new();
        self.skip_separators()?;

        if !matches!(self.tokens.peek(), Some(Ok(Token::RParen))) {
            loop {
                arguments.push(self.parse_expression(0)?);

                if matches!(self.tokens.peek(), Some(Ok(Token::RParen))) {
                    break;
                }

                self.expect_next_token(Token::Sep)?;
                self.skip_separators()?;

                if matches!(self.tokens.peek(), Some(Ok(Token::RParen))) {
                    break;
                }
            }
        }

        self.expect_next_token(Token::RParen)?;
        Ok(Expr::Call { name, arguments })
    }

    fn infix_binding_power(token: &Token<'input>) -> Option<(u8, u8, BinaryOp)> {
        match token {
            Token::Add => Some((10, 11, BinaryOp::Add)),
            Token::Sub => Some((10, 11, BinaryOp::Sub)),
            Token::Mul => Some((20, 21, BinaryOp::Mul)),
            Token::Div => Some((20, 21, BinaryOp::Div)),
            Token::Exp => Some((30, 30, BinaryOp::Exp)),
            _ => None,
        }
    }

    fn skip_separators(&mut self) -> Result<(), ParseError> {
        while matches!(self.tokens.peek(), Some(Ok(Token::Sep))) {
            self.next_token()?;
        }
        Ok(())
    }

    fn expect_next_token(&mut self, expected: Token<'input>) -> Result<(), ParseError> {
        let token = self.next_token()?;
        if token == expected {
            Ok(())
        } else {
            Err(self.unexpected(token, "unexpected token"))
        }
    }

    fn next_token(&mut self) -> Result<Token<'input>, ParseError> {
        self.tokens
            .next()
            .transpose()
            .map_err(ParseError::from)?
            .ok_or_else(|| ParseError {
                line: 0,
                message: "unexpected end of input".to_owned(),
            })
    }

    fn unexpected(&self, token: Token<'input>, message: &str) -> ParseError {
        ParseError {
            line: 0,
            message: format!("{message}: {token:?}"),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct ParseError {
    pub line: usize,
    pub message: String,
}
//endregion Parser

impl From<LexerError> for ParseError {
    fn from(error: LexerError) -> Self {
        Self {
            line: error.line,
            message: error.message,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Parser;
    use crate::ast::test_utils::*;

    #[test]
    fn parses_key_value_pairs() {
        let document = Parser::new("a = 1\nb=2\n\"key with spaces\" = 3")
            .parse()
            .expect("valid document");

        assert_pairs(
            document,
            &[("a", int(1)), ("b", int(2)), ("key with spaces", int(3))],
        );
    }

    #[test]
    fn parses_math_expressions() {
        let document = Parser::new(
            "sum = 1 + 2\ndifference = 5 - 2\nproduct = 2 * 3\nquotient = 8 / 2\nprecedence = 1 + 2 * 3\npower = 2 ^ 3 ^ 4\npositive = +1\nnegative = -2 ^ 2",
        )
        .parse()
        .expect("valid document");

        assert_pairs(
            document,
            &[
                ("sum", add(int(1), int(2))),
                ("difference", sub(int(5), int(2))),
                ("product", mul(int(2), int(3))),
                ("quotient", div(int(8), int(2))),
                ("precedence", add(int(1), mul(int(2), int(3)))),
                ("power", exp(int(2), exp(int(3), int(4)))),
                ("positive", op_pos(int(1))),
                ("negative", op_neg(exp(int(2), int(2)))),
            ],
        );
    }

    #[test]
    fn parses_function_calls_with_comma_or_newline_separators() {
        let document = Parser::new("value = foo(1, 2,3)\nother = bar(1\n2, 3\n)")
            .parse()
            .expect("valid document");

        assert_pairs(
            document,
            &[
                ("value", call("foo", vec![int(1), int(2), int(3)])),
                ("other", call("bar", vec![int(1), int(2), int(3)])),
            ],
        );
    }
}
