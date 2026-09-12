use crate::{
    lexer::token::{Spanned, Token},
    parser::{
        Parser, ParserResult,
        ast::{Expr, Identifier, InfixOp, KV, PrefixOp},
    },
};

impl<'input> Parser<'input> {
    /// Parses an expression.
    pub(super) fn parse_expr(
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
            Token::Id("if") if !self.next_is(&Token::Colon) => self.parse_if(),
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
        let condition = {
            let first = self.next_token()?;
            self.parse_expr(first, 0)?
        };

        let then = {
            // Skip {
            self.expect_next_token(&Token::LBrace)?;
            self.skip_separators()?;

            let first = self.next_token()?;
            let expr = self.parse_expr(first, 0)?;

            // Skip }
            self.skip_separators()?;
            self.expect_next_token(&Token::RBrace)?;

            expr
        };

        let r#else = {
            // Skip 'else'
            match self.next_token()? {
                Spanned {
                    value: Token::Id("else"),
                    ..
                } => {}
                token => return Err(Self::err_unexpected(&token, "`else`")),
            }

            // Skip {
            self.expect_next_token(&Token::LBrace)?;
            self.skip_separators()?;

            let first = self.next_token()?;
            let expr = self.parse_expr(first, 0)?;

            // Skip }
            self.skip_separators()?;
            self.expect_next_token(&Token::RBrace)?;

            expr
        };

        self.parse_postfix_op(Expr::If {
            condition: Box::new(condition),
            then: Box::new(then),
            r#else: Box::new(r#else),
        })
    }
}

#[cfg(test)]
mod tests {
    use insta::assert_snapshot;

    use super::Parser;
    use crate::test_utils::{dedent, fmt_snapshot_case, fmt_snapshot_cases};

    #[test]
    fn expect_asts() {
        let cases = vec![
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
            ("conditional expression", "if (true) { 1 } else { 2 }"),
            ("member call expression", "x.foo()"),
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
        let cases = [("empty expression", "()")];

        let cases = fmt_snapshot_cases(cases, |(label, input)| {
            let error = Parser::new(input)
                .parse_stmnts()
                .expect_err("expected a parser error");
            let error = error.to_string();
            fmt_snapshot_case(label, &[("input", input), ("error", &error)])
        });

        assert_snapshot!(cases);
    }
}
