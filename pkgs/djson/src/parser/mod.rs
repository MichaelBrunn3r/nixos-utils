#![allow(clippy::missing_errors_doc)]

pub mod ast;

use std::iter::Peekable;

use ast::AST;

use crate::{
    lexer::{
        Lexer, LexerError,
        token::{Spanned, Token},
    },
    parser::ast::{BinaryOp, Expr, Identifier, KV, Let, Statement, UnaryOp},
};

//region Parser
pub struct Parser<'input> {
    input: &'input str,
    tokens: Peekable<Lexer<'input>>,
    last_span: miette::SourceSpan,
    last_expression_span: miette::SourceSpan,
}

impl<'input> Parser<'input> {
    #[must_use]
    pub fn new(input: &'input str) -> Self {
        Self {
            input,
            tokens: Lexer::new(input).peekable(),
            last_span: (0, 0).into(),
            last_expression_span: (0, 0).into(),
        }
    }

    pub fn parse(mut self) -> ParserResult<AST<'input>> {
        let mut statements = Vec::new();
        self.skip_separators()?;

        while self.tokens.peek().is_some() {
            statements.push(self.parse_statement()?);
            self.skip_separators()?;
        }

        Ok(AST { statements })
    }

    fn parse_statement(&mut self) -> ParserResult<Statement<'input>> {
        if matches!(
            self.tokens.peek(),
            Some(Ok(Spanned {
                value: Token::Id("let"),
                ..
            }))
        ) {
            self.next_token()?;
            if matches!(
                self.tokens.peek(),
                Some(Ok(Spanned {
                    value: Token::Colon,
                    ..
                }))
            ) {
                self.next_token()?;
                return Ok(Statement::KV(KV {
                    key: "let",
                    expr: self.parse_expression(0)?,
                }));
            }
            return self.parse_let();
        }
        let expression = self.parse_expression(0)?;

        if !matches!(
            self.tokens.peek(),
            Some(Ok(Spanned {
                value: Token::Colon,
                ..
            }))
        ) {
            if matches!(
                self.tokens.peek(),
                Some(Ok(Spanned { value: token, .. })) if !matches!(token, Token::Sep)
            ) {
                let token = self.next_token()?;
                return Err(Self::unexpected(&token, "a separator or ':'"));
            }
            return Ok(Statement::Expr(expression));
        }

        let key = match expression {
            Expr::Id(Identifier::Simple(key)) | Expr::Str(key) => key,
            _ => self
                .input
                .get(
                    self.last_expression_span.offset()
                        ..self.last_expression_span.offset() + self.last_expression_span.len(),
                )
                .expect("expression spans must be valid input boundaries"),
        };

        self.next_token()?;
        Ok(Statement::KV(KV {
            key,
            expr: self.parse_expression(0)?,
        }))
    }

    fn parse_let(&mut self) -> ParserResult<Statement<'input>> {
        let name = match self.next_token()? {
            Spanned {
                value: Token::Id(value),
                ..
            } if value != "let" => value,
            token => return Err(Self::unexpected(&token, "an identifier")),
        };
        self.expect_next_token(&Token::Eq)?;
        Ok(Statement::Let(Let {
            name,
            expr: self.parse_expression(0)?,
        }))
    }

    fn parse_expression(&mut self, min_binding_power: u8) -> ParserResult<Expr<'input>> {
        let start = self
            .tokens
            .peek()
            .and_then(|result| result.as_ref().ok())
            .map_or(self.last_span, |token| token.span);
        let expression = self.parse_expression_inner(min_binding_power)?;
        let end = self.last_span.offset() + self.last_span.len();
        self.last_expression_span = (start.offset(), end.saturating_sub(start.offset())).into();
        Ok(expression)
    }

    fn parse_expression_inner(&mut self, min_binding_power: u8) -> ParserResult<Expr<'input>> {
        let mut left = self.parse_prefix()?;

        while let Some(Ok(token)) = self.tokens.peek() {
            let Some((left_binding_power, right_binding_power, operator)) =
                Self::infix_binding_power(&token.value)
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

    fn parse_prefix(&mut self) -> ParserResult<Expr<'input>> {
        let token = self.next_token()?;
        self.parse_prefix_token(token)
    }

    fn parse_prefix_token(&mut self, token: Spanned<Token<'input>>) -> ParserResult<Expr<'input>> {
        let Spanned { value, .. } = token;
        match value {
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
            token => Err(Self::unexpected(
                &Spanned {
                    value: token,
                    span: self.last_span,
                },
                "expression",
            )),
        }
    }

    fn parse_unary(&mut self, operator: UnaryOp) -> ParserResult<Expr<'input>> {
        let value = self.parse_expression(25)?;
        Ok(Expr::Unary {
            op: operator,
            value: Box::new(value),
        })
    }

    fn parse_postfix(&mut self, mut expression: Expr<'input>) -> ParserResult<Expr<'input>> {
        loop {
            expression = match self.tokens.peek() {
                Some(Ok(Spanned {
                    value: Token::Dot, ..
                })) => {
                    self.next_token()?;
                    let name = match self.next_token()? {
                        Spanned {
                            value: Token::Id(value),
                            ..
                        } => value,
                        token => {
                            return Err(Self::unexpected(&token, "an identifier"));
                        }
                    };
                    Expr::Access {
                        object: Box::new(expression),
                        name,
                    }
                }
                Some(Ok(Spanned {
                    value: Token::LParen,
                    ..
                })) => self.parse_call(expression)?,
                _ => return Ok(expression),
            };
        }
    }

    fn parse_call(&mut self, callee: Expr<'input>) -> ParserResult<Expr<'input>> {
        self.expect_next_token(&Token::LParen)?;
        let mut arguments = Vec::new();
        self.skip_separators()?;

        if !matches!(
            self.tokens.peek(),
            Some(Ok(Spanned {
                value: Token::RParen,
                ..
            }))
        ) {
            loop {
                arguments.push(self.parse_expression(0)?);

                if matches!(
                    self.tokens.peek(),
                    Some(Ok(Spanned {
                        value: Token::RParen,
                        ..
                    }))
                ) {
                    break;
                }

                self.expect_next_token(&Token::Sep)?;
                self.skip_separators()?;

                if matches!(
                    self.tokens.peek(),
                    Some(Ok(Spanned {
                        value: Token::RParen,
                        ..
                    }))
                ) {
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

    fn parse_list(&mut self) -> ParserResult<Expr<'input>> {
        let mut values = Vec::new();
        self.skip_separators()?;

        if !matches!(
            self.tokens.peek(),
            Some(Ok(Spanned {
                value: Token::RBracket,
                ..
            }))
        ) {
            loop {
                values.push(self.parse_expression(0)?);

                if matches!(
                    self.tokens.peek(),
                    Some(Ok(Spanned {
                        value: Token::RBracket,
                        ..
                    }))
                ) {
                    break;
                }

                self.expect_next_token(&Token::Sep)?;
                self.skip_separators()?;

                if matches!(
                    self.tokens.peek(),
                    Some(Ok(Spanned {
                        value: Token::RBracket,
                        ..
                    }))
                ) {
                    break;
                }
            }
        }

        self.expect_next_token(&Token::RBracket)?;
        self.parse_postfix(Expr::List(values))
    }

    fn parse_map(&mut self) -> ParserResult<Expr<'input>> {
        let mut entries = Vec::new();
        self.skip_separators()?;

        if !matches!(
            self.tokens.peek(),
            Some(Ok(Spanned {
                value: Token::RBrace,
                ..
            }))
        ) {
            loop {
                let key = match self.next_token()? {
                    Spanned {
                        value: Token::Id(value) | Token::Str(value),
                        ..
                    } => value,
                    token => return Err(Self::unexpected(&token, "a map key")),
                };
                self.expect_next_token(&Token::Colon)?;
                let expr = self.parse_expression(0)?;
                entries.push(KV { key, expr });

                if matches!(
                    self.tokens.peek(),
                    Some(Ok(Spanned {
                        value: Token::RBrace,
                        ..
                    }))
                ) {
                    break;
                }

                self.expect_next_token(&Token::Sep)?;
                self.skip_separators()?;

                if matches!(
                    self.tokens.peek(),
                    Some(Ok(Spanned {
                        value: Token::RBrace,
                        ..
                    }))
                ) {
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

    fn skip_separators(&mut self) -> ParserResult<()> {
        while matches!(
            self.tokens.peek(),
            Some(Ok(Spanned {
                value: Token::Sep,
                ..
            }))
        ) {
            self.next_token()?;
        }
        Ok(())
    }

    fn expect_next_token(&mut self, expected: &Token<'input>) -> ParserResult<()> {
        let token = self.next_token()?;
        if token.value == *expected {
            Ok(())
        } else {
            Err(Self::unexpected(&token, &format!("{expected:?}")))
        }
    }

    fn next_token(&mut self) -> ParserResult<Spanned<Token<'input>>> {
        let token = self
            .tokens
            .next()
            .transpose()
            .map_err(ParserError::from)?
            .ok_or(ParserError::UnexpectedEof {
                span: self.last_span,
            })?;
        self.last_span = token.span;
        Ok(token)
    }

    fn unexpected(token: &Spanned<Token<'input>>, expected: &str) -> ParserError {
        ParserError::UnexpectedToken {
            expected: expected.to_owned(),
            found: format!("{:?}", token.value),
            span: token.span,
        }
    }
}

//endregion Parser

//region ParserResult
pub type ParserResult<T> = Result<T, ParserError>;

#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum ParserError {
    #[error(transparent)]
    #[diagnostic(transparent)]
    Lexer(#[from] LexerError),

    #[error("unexpected end of input")]
    #[diagnostic(code(parser::unexpected_eof))]
    UnexpectedEof {
        #[label("input ends here")]
        span: miette::SourceSpan,
    },

    #[error("unexpected token")]
    #[diagnostic(code(parser::unexpected_token))]
    UnexpectedToken {
        expected: String,
        found: String,
        #[label("expected {expected}, got {found} instead")]
        span: miette::SourceSpan,
    },
}
//endregion ParserResult

#[cfg(test)]
mod tests {
    use insta::assert_snapshot;

    use super::Parser;
    use crate::test_utils::{dedent, fmt_diagnostic_case, fmt_snapshot_case, fmt_snapshot_cases};

    #[test]
    fn expect_asts() {
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
                "import expressions",
                "let std = import(\"std\")
                 std.math.sin(0)",
            ),
            ("keyword as top-level key", "let: value"),
            ("keyword as map key", "{let: 1}"),
            ("expression-shaped top-level key", "not_a_string(): 2"),
            ("let binding with map", "let value = { nested: 7 }"),
            (
                "let binding followed by a document field",
                "let value = 7
                 result: value",
            ),
        ];

        let cases = fmt_snapshot_cases(cases, |(label, input)| {
            let input = dedent(input);
            let document = Parser::new(&input).parse().expect("valid document");
            let input = input.replace('\n', "\n        ");
            let ast = document.pretty_string();
            format!("{label}\ninput: `{input}`\nast: {ast}")
        });

        assert_snapshot!(cases);
    }

    #[test]
    fn expect_errors() {
        let cases = [("adjacent top-level tokens", "key key")];

        let cases = fmt_snapshot_cases(cases, |(label, input)| {
            let error = Parser::new(input)
                .parse()
                .expect_err("expected a parser error");
            let error = error.to_string();
            fmt_snapshot_case(label, &[("input", input), ("error", &error)])
        });

        assert_snapshot!(cases);
    }

    #[test]
    fn expect_diagnostics() {
        let cases = [
            (
                "lexer error",
                "{
                    value: 9223372036854775808
                }",
            ),
            (
                "unexpected end of input",
                "before: 1
                 value:",
            ),
            (
                "unexpected token",
                "before: 1
                  value: )
                 after: 3",
            ),
        ];

        let cases = fmt_snapshot_cases(cases, |(label, input)| {
            let input = &dedent(input);
            let error = Parser::new(input)
                .parse()
                .expect_err("expected a parser error");
            fmt_diagnostic_case(label, input, error)
        });

        assert_snapshot!(cases);
    }
}
