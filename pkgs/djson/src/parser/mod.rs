#![allow(clippy::missing_errors_doc)]

pub mod ast;

use std::iter::Peekable;

use ast::AST;

use crate::{
    lexer::{
        Lexer, LexerError,
        token::{Spanned, Token},
    },
    parser::ast::{Expr, Identifier, InfixOp, KV, Let, PrefixOp, Statement},
};

//region Parser
pub struct Parser<'input> {
    tokens: Peekable<Lexer<'input>>,
    last_span: miette::SourceSpan,
    last_expression_span: miette::SourceSpan,
}

impl<'input> Parser<'input> {
    #[must_use]
    pub fn new(input: &'input str) -> Self {
        Self {
            tokens: Lexer::new(input).peekable(),
            last_span: (0, 0).into(),
            last_expression_span: (0, 0).into(),
        }
    }

    //region Parse statements
    /// Parses the complete input into an AST.
    pub fn parse_stmnts(mut self) -> ParserResult<AST<'input>> {
        let mut statements = Vec::new();
        self.skip_separators()?;

        while self.tokens.peek().is_some() {
            statements.push(self.parse_stmnt()?);
            self.skip_separators()?;
        }

        Ok(AST { statements })
    }

    /// Parses one statement, either a binding, key-value pair, or expression.
    fn parse_stmnt(&mut self) -> ParserResult<Statement<'input>> {
        let first = self.next_token()?;

        if matches!(first.value, Token::Id("let")) && !self.next_is(&Token::Colon) {
            return self.parse_let_stmnt();
        }

        let expression = self.parse_expr(first, 0)?;
        if !self.next_is(&Token::Colon) {
            if matches!(
                self.tokens.peek(),
                Some(Ok(Spanned { value: token, .. })) if !matches!(token, Token::Sep)
            ) {
                let token = self.next_token()?;
                return Err(Self::err_unexpected(&token, "a separator or ':'"));
            }
            return Ok(Statement::Expr(expression));
        }

        let (Expr::Id(Identifier::Simple(key)) | Expr::Str(key)) = expression else {
            return Err(ParserError::InvalidKey {
                span: self.last_expression_span,
            });
        };

        self.next_token()?;
        let first = self.next_token()?;
        Ok(Statement::KV(KV {
            key,
            expr: self.parse_expr(first, 0)?,
        }))
    }

    /// Parses a `let` binding statement.
    fn parse_let_stmnt(&mut self) -> ParserResult<Statement<'input>> {
        let name = match self.next_token()? {
            Spanned {
                value: Token::Id(value),
                ..
            } if value != "let" => value,
            Spanned {
                value: Token::Id("let"),
                span,
            } => return Err(ParserError::ReservedIdentifier { span }),
            token => return Err(Self::err_unexpected(&token, "an identifier")),
        };
        self.expect_next_token(&Token::Eq)?;
        let first = self.next_token()?;
        Ok(Statement::Let(Let {
            name,
            expr: self.parse_expr(first, 0)?,
        }))
    }
    //endregion Parse statements

    //region Parse expression
    /// Parses an expression.
    fn parse_expr(
        &mut self,
        first: Spanned<Token<'input>>,
        min_bp: u8,
    ) -> ParserResult<Expr<'input>> {
        let start = first.span;
        let mut lhs = self.parse_lhs(first)?;

        while let Some(Ok(token)) = self.tokens.peek() {
            let Some((lhs_bp, rhs_bp, op)) = Self::infix_binding_power(&token.value) else {
                break;
            };

            if lhs_bp < min_bp {
                break;
            }

            self.next_token()?;
            lhs = self.parse_rhs_extension(lhs, rhs_bp, op)?;
        }

        let end = self.last_span.offset() + self.last_span.len();
        self.last_expression_span = (start.offset(), end.saturating_sub(start.offset())).into();
        Ok(lhs)
    }

    /// Parses the left-hand side of an expression.
    fn parse_lhs(&mut self, token: Spanned<Token<'input>>) -> ParserResult<Expr<'input>> {
        let Spanned { value, .. } = token;
        match value {
            Token::Bool(value) => self.parse_postfix_op(Expr::Bool(value)),
            Token::Int(value) => self.parse_postfix_op(Expr::Int(value)),
            Token::Float(value) => self.parse_postfix_op(Expr::Float(value)),
            Token::Str(value) => self.parse_postfix_op(Expr::Str(value)),
            Token::LBracket => self.parse_list(),
            Token::LBrace => self.parse_map(),
            Token::Id("if") if self.next_is(&Token::LParen) => self.parse_if(),
            Token::Id(value) => self.parse_postfix_op(Expr::Id(Identifier::Simple(value))),
            Token::LParen => {
                let first = self.next_token()?;
                let expression = self.parse_expr(first, 0)?;
                self.expect_next_token(&Token::RParen)?;
                self.parse_postfix_op(expression)
            }
            Token::Add => self.parse_prefix_op(PrefixOp::Positive),
            Token::Sub => self.parse_prefix_op(PrefixOp::Negative),
            token => Err(Self::err_unexpected(
                &Spanned {
                    value: token,
                    span: self.last_span,
                },
                "expression",
            )),
        }
    }

    fn parse_rhs_extension(
        &mut self,
        lhs: Expr<'input>,
        rhs_bp: u8,
        op: InfixOp,
    ) -> ParserResult<Expr<'input>> {
        let first = self.next_token()?;
        let right = self.parse_expr(first, rhs_bp)?;
        Ok(Expr::Binary {
            left: Box::new(lhs),
            op,
            right: Box::new(right),
        })
    }

    /// Parses a prefix operator and its operand.
    fn parse_prefix_op(&mut self, op: PrefixOp) -> ParserResult<Expr<'input>> {
        let first = self.next_token()?;
        let value = self.parse_expr(first, 25)?;
        Ok(Expr::Unary {
            op,
            value: Box::new(value),
        })
    }

    fn parse_postfix_op(&mut self, mut expr: Expr<'input>) -> ParserResult<Expr<'input>> {
        loop {
            expr = match self.tokens.peek() {
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
                            return Err(Self::err_unexpected(&token, "an identifier"));
                        }
                    };
                    Expr::Access {
                        object: Box::new(expr),
                        name,
                    }
                }
                Some(Ok(Spanned {
                    value: Token::LParen,
                    ..
                })) => self.parse_call(expr)?,
                _ => return Ok(expr),
            };
        }
    }

    /// Parses a function call and its arguments.
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
                let first = self.next_token()?;
                arguments.push(self.parse_expr(first, 0)?);

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

    /// Parses a list literal and its elements.
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
                let first = self.next_token()?;
                values.push(self.parse_expr(first, 0)?);

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
        self.parse_postfix_op(Expr::List(values))
    }

    /// Parses a map literal and its entries.
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
                    token => return Err(Self::err_unexpected(&token, "a map key")),
                };
                self.expect_next_token(&Token::Colon)?;
                let first = self.next_token()?;
                let expr = self.parse_expr(first, 0)?;
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
        self.parse_postfix_op(Expr::Map(entries))
    }

    /// Parses a conditional expression with `then` and `else` branches.
    fn parse_if(&mut self) -> ParserResult<Expr<'input>> {
        self.expect_next_token(&Token::LParen)?;
        let first = self.next_token()?;
        let condition = self.parse_expr(first, 0)?;
        self.expect_next_token(&Token::RParen)?;
        self.expect_next_token(&Token::LBrace)?;
        self.skip_separators()?;
        let first = self.next_token()?;
        let then_branch = self.parse_expr(first, 0)?;
        self.skip_separators()?;
        self.expect_next_token(&Token::RBrace)?;
        match self.next_token()? {
            Spanned {
                value: Token::Id("else"),
                ..
            } => {}
            token => return Err(Self::err_unexpected(&token, "`else`")),
        }
        self.expect_next_token(&Token::LBrace)?;
        self.skip_separators()?;
        let first = self.next_token()?;
        let else_branch = self.parse_expr(first, 0)?;
        self.skip_separators()?;
        self.expect_next_token(&Token::RBrace)?;
        self.parse_postfix_op(Expr::If {
            condition: Box::new(condition),
            then_branch: Box::new(then_branch),
            else_branch: Box::new(else_branch),
        })
    }
    //endregion Parse expression

    fn next_is(&mut self, expected: &Token<'input>) -> bool {
        matches!(self.tokens.peek(), Some(Ok(token)) if &token.value == expected)
    }

    const fn infix_binding_power(token: &Token<'input>) -> Option<(u8, u8, InfixOp)> {
        match token {
            Token::Add => Some((10, 11, InfixOp::Add)),
            Token::Sub => Some((10, 11, InfixOp::Sub)),
            Token::Mul => Some((20, 21, InfixOp::Mul)),
            Token::Div => Some((20, 21, InfixOp::Div)),
            Token::Exp => Some((30, 30, InfixOp::Exp)),
            Token::Equal => Some((5, 6, InfixOp::Equal)),
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
            Err(Self::err_unexpected(&token, &format!("{expected:?}")))
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

    fn err_unexpected(token: &Spanned<Token<'input>>, expected: &str) -> ParserError {
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

    #[error("reserved identifier")]
    #[diagnostic(code(parser::reserved_identifier))]
    ReservedIdentifier {
        #[label("`let` is not allowed as a binding name")]
        span: miette::SourceSpan,
    },

    #[error("invalid key")]
    #[diagnostic(code(parser::invalid_key))]
    InvalidKey {
        #[label("keys must be identifiers or strings")]
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
            ("string expression", "\"hello\""),
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
                "expression continuation after let binding",
                "let x = 1
                  x + 1",
            ),
            ("conditional expression", "if (true) { 1 } else { 2 }"),
            ("member call expression", "x.foo()"),
            (
                "import expressions",
                "let std = import(\"std\")
                 std.math.sin(0)",
            ),
            (
                "keywords as top-level key",
                "let: 1
                 if: 2
                 else: 3",
            ),
            (
                "keywords as map key",
                "{
                    let: 1
                    if: 2
                    else: 3
                }",
            ),
            (
                "quoted expression-shaped top-level key",
                "\"not_a_string()\": 2",
            ),
            ("let binding with map", "let value = { nested: 7 }"),
            (
                "let binding followed by a document field",
                "let value = 7
                 result: value",
            ),
        ];

        let cases = fmt_snapshot_cases(cases, |(label, input)| {
            let input = dedent(input);
            let document = Parser::new(&input).parse_stmnts().expect("valid document");
            let input = input.replace('\n', "\n        ");
            let ast = document.pretty_string();
            format!("{label}\ninput: `{input}`\nast: {ast}")
        });

        assert_snapshot!(cases);
    }

    #[test]
    fn expect_errors() {
        let cases = [
            ("adjacent top-level tokens", "key key"),
            ("arithmetic expression as key", "1 + 1: 1"),
            ("call expression as key", "x(): 1"),
            ("member expression as key", "x.foo(): 1"),
            ("multi-word unquoted key", "let there be rain: 1"),
            ("reserved identifier", "let let = 1"),
            ("non-identifier binding name", "let 1 = 1"),
        ];

        let cases = fmt_snapshot_cases(cases, |(label, input)| {
            let error = Parser::new(input)
                .parse_stmnts()
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
            ("reserved identifier", "let let = 1"),
        ];

        let cases = fmt_snapshot_cases(cases, |(label, input)| {
            let input = &dedent(input);
            let error = Parser::new(input)
                .parse_stmnts()
                .expect_err("expected a parser error");
            fmt_diagnostic_case(label, input, error)
        });

        assert_snapshot!(cases);
    }
}
