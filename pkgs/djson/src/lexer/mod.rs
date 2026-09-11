#![allow(clippy::cast_precision_loss)]

use miette::SourceSpan;

use crate::lexer::token::{Spanned, Token};
pub mod token;

pub struct Lexer<'input> {
    input: &'input str,
    pos: usize,
    buffer: String,
}

impl<'input> Lexer<'input> {
    #[must_use]
    pub const fn new(input: &'input str) -> Self {
        Self {
            input,
            pos: 0,
            buffer: String::new(),
        }
    }
}

impl<'input> Lexer<'input> {
    fn eat(&mut self) -> Option<char> {
        let character = self.peek()?;
        self.pos += character.len_utf8();
        Some(character)
    }

    //region Lookahead
    fn peek(&self) -> Option<char> {
        self.input[self.pos..].chars().next()
    }

    fn starts_with(&self, pattern: &str) -> bool {
        self.input[self.pos..].starts_with(pattern)
    }

    //endregion Lookahead

    //region Skip
    fn skip_n(&mut self, len: usize) {
        for _ in 0..len {
            self.eat();
        }
    }

    fn skip_while(&mut self, predicate: impl Fn(char) -> bool) {
        while self.peek().is_some_and(&predicate) {
            self.eat();
        }
    }

    fn skip_ignored(&mut self) -> Result<bool, LexerError> {
        let mut skipped_over_line = false;
        loop {
            self.skip_while(|character| character.is_whitespace() && character != '\n'); // Skip whitespace
            if self.starts_with("//") || self.starts_with("#") {
                self.skip_while(|character| character != '\n'); // Skip line comment
                return Ok(skipped_over_line);
            } else if self.starts_with("/*") {
                skipped_over_line |= self.skip_block_comment()?;
            } else {
                return Ok(skipped_over_line);
            }
        }
    }

    fn skip_block_comment(&mut self) -> Result<bool, LexerError> {
        let start = self.pos;
        self.skip_n("/*".len());

        let mut line_changed = false;
        while self.peek().is_some() {
            if self.starts_with("*/") {
                self.skip_n("*/".len());
                return Ok(line_changed);
            }
            line_changed |= self.peek() == Some('\n');
            self.eat();
        }

        Err(LexerError::UnterminatedBlockComment {
            span: (start, 1).into(),
        })
    }
    //endregion Skip

    //region Read
    fn read_string(&mut self, delimiter: char) -> LexerResult<'input> {
        let start = self.pos;
        self.eat();
        let content_start = self.pos;

        while let Some(c) = self.peek() {
            if c == delimiter {
                let end = self.pos;
                self.eat();
                return Ok(Spanned {
                    value: Token::Str(&self.input[content_start..end]),
                    span: (start, self.pos - start + delimiter.len_utf8()).into(),
                });
            }
            if c == '\\' {
                self.skip_n("\\n".len());
                continue;
            }
            self.eat();
        }

        Err(LexerError::UnterminatedString {
            span: (start, delimiter.len_utf8()).into(),
        })
    }

    fn read_number(&mut self) -> LexerResult<'input> {
        let start = self.pos;
        let mut has_decimals = false;

        let literal = loop {
            let Some(character) = self.peek() else {
                break &self.input[start..self.pos];
            };

            match character {
                '0'..='9' => {
                    self.eat();
                }
                '.' if !has_decimals
                    && self.input[self.pos..]
                        .chars()
                        .nth(1)
                        .is_some_and(|character| {
                            character.is_ascii_digit() || matches!(character, '_' | '\'')
                        }) =>
                {
                    has_decimals = true;
                    self.eat();
                }
                '_' | '\'' => {
                    let (norm_has_decimals, norm_literal) =
                        self.read_normalized_number(start, has_decimals)?;
                    has_decimals |= norm_has_decimals;
                    break norm_literal;
                }
                _ => break &self.input[start..self.pos],
            }
        };

        if has_decimals {
            let value = literal
                .parse::<f64>()
                .expect("lexer only accepts valid float literals");
            Ok(Spanned {
                value: Token::Float(value),
                span: (start, self.pos - start).into(),
            })
        } else {
            literal
                .parse::<i64>()
                .map(|value| Spanned {
                    value: Token::Int(value),
                    span: (start, self.pos - start).into(),
                })
                .map_err(|error| {
                    debug_assert!(matches!(error.kind(), std::num::IntErrorKind::PosOverflow));
                    Self::err_invalid_number(
                        start,
                        self.pos - start,
                        InvalidNumberReason::IntOverflow,
                    )
                })
        }
    }

    fn read_normalized_number(
        &mut self,
        start: usize,
        mut has_decimals: bool,
    ) -> Result<(bool, &str), LexerError> {
        self.buffer.clear();
        self.buffer.push_str(&self.input[start..self.pos]);

        while let Some(character) = self.peek() {
            match character {
                '0'..='9' => {
                    self.buffer.push(character);
                    self.eat();
                }
                '.' if !has_decimals => {
                    has_decimals = true;
                    self.buffer.push(character);
                    self.eat();
                }
                '_' | '\'' => {
                    let separator_start = self.pos;
                    self.skip_while(|next| matches!(next, '_' | '\''));
                    let separator_length = self.pos - separator_start;
                    if separator_length != 1
                        || !self.peek().is_some_and(|next| next.is_ascii_digit())
                    {
                        return Err(Self::err_invalid_number(
                            separator_start,
                            separator_length,
                            InvalidNumberReason::InvalidSeparator,
                        ));
                    }
                }
                _ => break,
            }
        }

        Ok((has_decimals, self.buffer.as_str()))
    }

    fn read_identifier(&mut self) -> Token<'input> {
        let start = self.pos;

        while self.peek().is_some_and(|character| {
            !character.is_whitespace()
                && !matches!(
                    character,
                    ',' | '+'
                        | '-'
                        | '^'
                        | '*'
                        | '/'
                        | '='
                        | ':'
                        | '('
                        | ')'
                        | '['
                        | ']'
                        | '{'
                        | '}'
                        | '.'
                        | '"'
                        | '\''
                        | '#'
                )
        }) {
            self.eat();
        }

        let identifier = &self.input[start..self.pos];
        match identifier {
            "true" => Token::Bool(true),
            "false" => Token::Bool(false),
            _ => Token::Id(identifier),
        }
    }
    //endregion Read

    //region Error
    fn err_invalid_number(start: usize, length: usize, reason: InvalidNumberReason) -> LexerError {
        LexerError::InvalidNumber {
            reason,
            span: (start, length).into(),
        }
    }
    //endregion Error
}

impl<'input> Iterator for Lexer<'input> {
    type Item = LexerResult<'input>;

    fn next(&mut self) -> Option<Self::Item> {
        let skipped_over_line = match self.skip_ignored() {
            Ok(skipped_over_line) => skipped_over_line,
            Err(error) => return Some(Err(error)),
        };
        if skipped_over_line {
            return Some(Ok(Spanned {
                value: Token::Sep,
                span: (self.pos, 0).into(),
            })); // Emit a separator token if we skipped across lines
        }

        let start = self.pos;
        let c = self.peek()?;

        let token = match c {
            ',' | '\n' => Token::Sep,
            '(' => Token::LParen,
            ')' => Token::RParen,
            '[' => Token::LBracket,
            ']' => Token::RBracket,
            '{' => Token::LBrace,
            '}' => Token::RBrace,
            '.' => Token::Dot,
            '+' => Token::Add,
            '-' => Token::Sub,
            '^' => Token::Exp,
            '*' => {
                if self.starts_with("**") {
                    self.eat();
                    Token::Exp
                } else {
                    Token::Mul
                }
            }
            '/' => Token::Div,
            ':' => Token::Colon,
            '=' => {
                if self.starts_with("==") {
                    self.eat();
                    Token::Equal
                } else {
                    Token::Eq
                }
            }
            '"' | '\'' => return Some(self.read_string(c)),
            '0'..='9' => return Some(self.read_number()),
            _ => {
                let value = self.read_identifier();
                return Some(Ok(Spanned {
                    value,
                    span: (start, self.pos - start).into(),
                }));
            }
        };

        self.eat();
        Some(Ok(Spanned {
            value: token,
            span: (start, self.pos - start).into(),
        }))
    }
}

//region LexerResult
pub type LexerResult<'input> = Result<Spanned<Token<'input>>, LexerError>;

#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum LexerError {
    #[error("invalid number")]
    #[diagnostic(code(lexer::invalid_number))]
    InvalidNumber {
        reason: InvalidNumberReason,
        #[label("{reason}")]
        span: SourceSpan,
    },

    #[error("unterminated string")]
    #[diagnostic(code(lexer::unterminated_string))]
    UnterminatedString {
        #[label("string starts here")]
        span: SourceSpan,
    },

    #[error("unterminated block comment")]
    #[diagnostic(code(lexer::unterminated_comment))]
    UnterminatedBlockComment {
        #[label("comment starts here")]
        span: SourceSpan,
    },
}

#[derive(Debug, thiserror::Error)]
pub enum InvalidNumberReason {
    #[error("integer: too large")]
    IntOverflow,

    #[error("invalid separator")]
    InvalidSeparator,
}
//endregion LexerResult

#[cfg(test)]
mod tests {
    use insta::assert_snapshot;

    use super::Lexer;
    use crate::test_utils::{dedent, fmt_diagnostic_case, fmt_snapshot_case, fmt_snapshot_cases};

    #[test]
    #[allow(clippy::approx_constant)]
    fn expect_tokens() {
        let cases = vec![
            ("operators and punctuation", "+ - * / ^ ** = == :"),
            ("integer", "0 42 0000123 123456789"),
            ("float", "3.14 -0.5 1.0"),
            ("member access after numbers", "1.floor() 1.0.floor()"),
            ("number separators", "1_000 3'000.14 1._000"),
            ("identifiers around operators", "foo + bar/baz"),
            ("single quote strings and number separators", "'text' 1'000"),
            (
                "strings",
                r#"'single' "double quote" "string // containing /* comments */" "#,
            ),
            (
                "line and block comments",
                "1// line comment 2 3 4
                   /* block
                   1 2 3
                   4 5 6
                   comment */2
                         3 /* inline block */ 4",
            ),
            (
                "hash line comments and shebang",
                "#!/usr/bin/env djson
                   1 # trailing comment
                   foo#comment
                   2 #",
            ),
        ];

        let cases = fmt_snapshot_cases(cases, |(label, input)| {
            let tokens = Lexer::new(input)
                .map(Result::unwrap)
                .map(|token| token.value)
                .collect::<Vec<_>>();
            fmt_snapshot_case(
                label,
                &[("input", input), ("tokens", &format!("{tokens:?}"))],
            )
        });

        assert_snapshot!(cases);
    }

    #[test]
    fn expect_errors() {
        let cases = vec![
            ("comment: unterminated block", "/* comment"),
            // invalid numbers
            ("num: int overflow", "9223372036854775808"),
            ("num: trailing separator", "100_"),
            ("num: separator sequence", "1__000"),
            ("num: mixed separator sequence", "1_'000"),
            // invalid strings
            ("str: unterminated `\"`", "\"str"),
            ("str: unterminated `\'`", "'str"),
        ];

        let cases = fmt_snapshot_cases(cases, |(label, input)| {
            let error = Lexer::new(input)
                .next()
                .expect("expected a lexer result")
                .expect_err("expected a lexer error");
            let error = error.to_string();
            fmt_snapshot_case(label, &[("input", input), ("error", &error)])
        });

        assert_snapshot!(cases);
    }

    #[test]
    fn expect_diagnostics() {
        let cases = [
            (
                "int overflow",
                "{
                    foo: 1
                    overflow: 9223372036854775808
                    bar: 2
                }",
            ),
            (
                "int invalid separator",
                "{
                    foo: 1
                    sep: 123___''_456
                    bar: 2
                }",
            ),
            (
                "unterminated string",
                "{
                    foo: 1
                    unterminated: \"hello world
                    bar: 2
                }",
            ),
            (
                "unterminated block comment",
                "{
                    foo: 1
                    unterminated: \"block\" /* comment
                    bar: 2
                }",
            ),
        ];

        let cases = fmt_snapshot_cases(cases, |(label, input)| {
            let input = &dedent(input);
            let error = Lexer::new(input)
                .find_map(Result::err)
                .expect("expected a lexer error");
            fmt_diagnostic_case(label, input, error)
        });

        assert_snapshot!(cases);
    }
}
