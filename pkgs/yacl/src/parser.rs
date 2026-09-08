#![allow(clippy::missing_errors_doc)]

use std::iter::Peekable;

use crate::ast::{AST, BinaryOp, Expr, Identifier, KV, Let, Statement, UnaryOp, Use};
use crate::lexer::{Lexer, LexerError, Token};

//region Parser
pub struct Parser<'input> {
    tokens: Peekable<Lexer<'input>>,
}

impl<'input> Parser<'input> {
    #[must_use]
    pub fn new(input: &'input str) -> Self {
        Self {
            tokens: Lexer::new(input).peekable(),
        }
    }

    pub fn parse(mut self) -> Result<AST<'input>, ParseError> {
        let mut statements = Vec::new();
        self.skip_separators()?;

        while self.tokens.peek().is_some() {
            statements.push(self.parse_statement()?);
            self.skip_separators()?;
        }

        Ok(AST { statements })
    }

    fn parse_statement(&mut self) -> Result<Statement<'input>, ParseError> {
        if matches!(self.tokens.peek(), Some(Ok(Token::Id("let")))) {
            return self.parse_let();
        }
        if matches!(self.tokens.peek(), Some(Ok(Token::Id("use")))) {
            return self.parse_use();
        }

        let expression = self.parse_expression(0)?;

        if !matches!(self.tokens.peek(), Some(Ok(Token::Colon))) {
            return Ok(Statement::Expr(expression));
        }

        let key = match expression {
            Expr::Id(Identifier::Simple(key)) | Expr::Str(key) => key,
            expression => {
                return Err(ParseError {
                    line: 0,
                    message: format!("expected a key: {expression:?}"),
                });
            }
        };

        self.next_token()?;
        Ok(Statement::KV(KV {
            key,
            expr: self.parse_expression(0)?,
        }))
    }

    fn parse_let(&mut self) -> Result<Statement<'input>, ParseError> {
        self.next_token()?;
        let name = match self.next_token()? {
            Token::Id(value) if value != "let" => value,
            token => return Err(Self::unexpected(&token, "expected an identifier")),
        };
        self.expect_next_token(&Token::Eq)?;
        Ok(Statement::Let(Let {
            name,
            expr: self.parse_expression(0)?,
        }))
    }

    fn parse_use(&mut self) -> Result<Statement<'input>, ParseError> {
        self.next_token()?;
        let mut path = vec![match self.next_token()? {
            Token::Id(value) => value,
            token => return Err(Self::unexpected(&token, "expected an identifier")),
        }];
        let mut wildcard = false;

        while matches!(self.tokens.peek(), Some(Ok(Token::Dot))) {
            self.next_token()?;
            if matches!(self.tokens.peek(), Some(Ok(Token::Mul))) {
                self.next_token()?;
                wildcard = true;
                break;
            }
            path.push(match self.next_token()? {
                Token::Id(value) => value,
                token => return Err(Self::unexpected(&token, "expected an identifier")),
            });
        }

        Ok(Statement::Use(Use { path, wildcard }))
    }

    fn parse_expression(&mut self, min_binding_power: u8) -> Result<Expr<'input>, ParseError> {
        let mut left = self.parse_prefix()?;

        while let Some(Ok(token)) = self.tokens.peek() {
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
        let token = self.next_token()?;
        self.parse_prefix_token(token)
    }

    fn parse_prefix_token(&mut self, token: Token<'input>) -> Result<Expr<'input>, ParseError> {
        match token {
            Token::Bool(value) => self.parse_postfix(Expr::Bool(value)),
            Token::Int(value) => self.parse_postfix(Expr::Int(value)),
            Token::Float(value) => self.parse_postfix(Expr::Float(value)),
            Token::Str(value) => self.parse_postfix(Expr::Str(value)),
            Token::LBracket => self.parse_list(),
            Token::LBrace => self.parse_map(),
            Token::Id(value) => self.parse_postfix(Expr::Id(Identifier::Simple(value))),
            Token::LParen => {
                let expression = self.parse_expression(0)?;
                self.expect_next_token(&Token::RParen)?;
                self.parse_postfix(expression)
            }
            Token::Add => self.parse_unary(UnaryOp::Positive),
            Token::Sub => self.parse_unary(UnaryOp::Negative),
            token => Err(Self::unexpected(&token, "expected an expression")),
        }
    }

    fn parse_unary(&mut self, operator: UnaryOp) -> Result<Expr<'input>, ParseError> {
        let value = self.parse_expression(25)?;
        Ok(Expr::Unary {
            op: operator,
            value: Box::new(value),
        })
    }

    fn parse_postfix(&mut self, mut expression: Expr<'input>) -> Result<Expr<'input>, ParseError> {
        loop {
            expression = match self.tokens.peek() {
                Some(Ok(Token::Dot)) => {
                    self.next_token()?;
                    let name = match self.next_token()? {
                        Token::Id(value) => value,
                        token => {
                            return Err(Self::unexpected(&token, "expected an identifier"));
                        }
                    };
                    Expr::Access {
                        object: Box::new(expression),
                        name,
                    }
                }
                Some(Ok(Token::LParen)) => self.parse_call(expression)?,
                _ => return Ok(expression),
            };
        }
    }

    fn parse_call(&mut self, callee: Expr<'input>) -> Result<Expr<'input>, ParseError> {
        self.expect_next_token(&Token::LParen)?;
        let mut arguments = Vec::new();
        self.skip_separators()?;

        if !matches!(self.tokens.peek(), Some(Ok(Token::RParen))) {
            loop {
                arguments.push(self.parse_expression(0)?);

                if matches!(self.tokens.peek(), Some(Ok(Token::RParen))) {
                    break;
                }

                self.expect_next_token(&Token::Sep)?;
                self.skip_separators()?;

                if matches!(self.tokens.peek(), Some(Ok(Token::RParen))) {
                    break;
                }
            }
        }

        self.expect_next_token(&Token::RParen)?;
        Ok(Expr::Call {
            callee: Box::new(callee),
            arguments,
        })
    }

    fn parse_list(&mut self) -> Result<Expr<'input>, ParseError> {
        let mut values = Vec::new();
        self.skip_separators()?;

        if !matches!(self.tokens.peek(), Some(Ok(Token::RBracket))) {
            loop {
                values.push(self.parse_expression(0)?);

                if matches!(self.tokens.peek(), Some(Ok(Token::RBracket))) {
                    break;
                }

                self.expect_next_token(&Token::Sep)?;
                self.skip_separators()?;

                if matches!(self.tokens.peek(), Some(Ok(Token::RBracket))) {
                    break;
                }
            }
        }

        self.expect_next_token(&Token::RBracket)?;
        self.parse_postfix(Expr::List(values))
    }

    fn parse_map(&mut self) -> Result<Expr<'input>, ParseError> {
        let mut entries = Vec::new();
        self.skip_separators()?;

        if !matches!(self.tokens.peek(), Some(Ok(Token::RBrace))) {
            loop {
                let key = match self.next_token()? {
                    Token::Id(value) | Token::Str(value) => value,
                    token => return Err(Self::unexpected(&token, "expected a map key")),
                };
                self.expect_next_token(&Token::Colon)?;
                let expr = self.parse_expression(0)?;
                entries.push(KV { key, expr });

                if matches!(self.tokens.peek(), Some(Ok(Token::RBrace))) {
                    break;
                }

                self.expect_next_token(&Token::Sep)?;
                self.skip_separators()?;

                if matches!(self.tokens.peek(), Some(Ok(Token::RBrace))) {
                    break;
                }
            }
        }

        self.expect_next_token(&Token::RBrace)?;
        self.parse_postfix(Expr::Map(entries))
    }

    const fn infix_binding_power(token: &Token<'input>) -> Option<(u8, u8, BinaryOp)> {
        match token {
            Token::Add => Some((10, 11, BinaryOp::Add)),
            Token::Sub => Some((10, 11, BinaryOp::Sub)),
            Token::Mul => Some((20, 21, BinaryOp::Mul)),
            Token::Div => Some((20, 21, BinaryOp::Div)),
            Token::Exp => Some((30, 30, BinaryOp::Exp)),
            Token::Equal => Some((5, 6, BinaryOp::Equal)),
            _ => None,
        }
    }

    fn skip_separators(&mut self) -> Result<(), ParseError> {
        while matches!(self.tokens.peek(), Some(Ok(Token::Sep))) {
            self.next_token()?;
        }
        Ok(())
    }

    fn expect_next_token(&mut self, expected: &Token<'input>) -> Result<(), ParseError> {
        let token = self.next_token()?;
        if token == *expected {
            Ok(())
        } else {
            Err(Self::unexpected(&token, "unexpected token"))
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

    fn unexpected(token: &Token<'input>, message: &str) -> ParseError {
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
    use insta::assert_snapshot;

    use super::Parser;
    use crate::ast::{Expr, Identifier, Let, Statement};
    use crate::test_utils::dedent;

    #[test]
    fn parses_cases() {
        let cases = vec![
            (
                "key value pairs",
                "a: 1
                  b: 2
                  \"key with spaces\": 3",
            ),
            ("expression statements", "1 + 2 * 3"),
            (
                "math expressions",
                "sum: 1 + 2
                 difference: 5 - 2
                 product: 2 * 3
                 quotient: 8 / 2
                 precedence: 1 + 2 * 3
                 power: 2 ^ 3 ^ 4
                 positive: +1
                 negative: -2 ^ 2",
            ),
            (
                "function calls",
                "inline: foo(1, 2,3)
                 multiline: bar(
                    1
                    2,
                    3
                 )",
            ),
            (
                "use statements",
                "use std
                 use std.sin
                 use std.*",
            ),
        ];

        let cases = cases
            .into_iter()
            .map(|(label, input)| {
                let input = dedent(input);
                let document = Parser::new(&input).parse().expect("valid document");
                let input = input.replace('\n', "\n        ");
                let ast = document.pretty_string();
                format!("{label}\ninput: `{input}`\nast: {ast}")
            })
            .collect::<Vec<_>>()
            .join("\n\n");

        assert_snapshot!(cases);
    }

    #[test]
    fn parses_let_binding() {
        let ast = Parser::new("let value = { nested: 7 }")
            .parse()
            .expect("let binding should parse");
        assert_eq!(
            ast.statements,
            vec![Statement::Let(Let {
                name: "value",
                expr: Expr::Map(vec![crate::ast::KV {
                    key: "nested",
                    expr: Expr::Int(7),
                }]),
            })]
        );

        let ast = Parser::new("let value = 7\nresult: value")
            .parse()
            .expect("let binding followed by a document field should parse");
        assert!(matches!(
            &ast.statements[1],
            Statement::KV(crate::ast::KV {
                key: "result",
                expr: Expr::Id(Identifier::Simple("value")),
            })
        ));
    }
}
