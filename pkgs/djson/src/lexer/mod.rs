#![allow(clippy::cast_precision_loss)]

use std::num::IntErrorKind;

use miette::SourceSpan;

use crate::lexer::token::Token;
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
            if self.starts_with("//") {
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
            span: (start, "/*".len()).into(),
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
                return Ok(Token::Str(&self.input[content_start..end]));
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

    fn err_invalid_number(&self, start: usize, reason: InvalidNumberReason) -> LexerError {
        LexerError::InvalidNumber {
            reason,
            span: (start, self.pos - start).into(),
        }
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
                '.' if self.input[self.pos..]
                    .chars()
                    .nth(1)
                    .is_some_and(|character| {
                        character.is_ascii_digit() || matches!(character, '_' | '\'')
                    }) =>
                {
                    if has_decimals {
                        return Err(self.err_invalid_number(
                            start,
                            InvalidNumberReason::MultipleDecimalPoints,
                        ));
                    }

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
            literal
                .parse::<f64>()
                .map(Token::Float)
                .map_err(|_| self.err_invalid_number(start, InvalidNumberReason::ParseFloat))
        } else {
            literal.parse::<i64>().map(Token::Int).map_err(|error| {
                self.err_invalid_number(start, InvalidNumberReason::ParseInt(*error.kind()))
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
                '.' => {
                    if has_decimals {
                        return Err(self.err_invalid_number(
                            start,
                            InvalidNumberReason::MultipleDecimalPoints,
                        ));
                    }

                    has_decimals = true;
                    self.buffer.push(character);
                    self.eat();
                }
                '_' | '\'' => {
                    self.eat();
                    if !self.peek().is_some_and(|next| next.is_ascii_digit()) {
                        return Err(
                            self.err_invalid_number(start, InvalidNumberReason::InvalidSeparator)
                        );
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
}

impl<'input> Iterator for Lexer<'input> {
    type Item = LexerResult<'input>;

    fn next(&mut self) -> Option<Self::Item> {
        let skipped_over_line = match self.skip_ignored() {
            Ok(skipped_over_line) => skipped_over_line,
            Err(error) => return Some(Err(error)),
        };
        if skipped_over_line {
            return Some(Ok(Token::Sep)); // Emit a separator token if we skipped across lines
        }

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
            _ => return Some(Ok(self.read_identifier())),
        };

        self.eat();
        Some(Ok(token))
    }
}

//region LexerResult
pub type LexerResult<'input> = Result<Token<'input>, LexerError>;

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
    #[error("integer: {}", int_error_reason(*.0))]
    ParseInt(IntErrorKind),

    #[error("float")]
    ParseFloat,

    #[error("multiple decimal points")]
    MultipleDecimalPoints,

    #[error("invalid separator")]
    InvalidSeparator,
}

const fn int_error_reason(kind: IntErrorKind) -> &'static str {
    match kind {
        IntErrorKind::Empty => "empty",
        IntErrorKind::InvalidDigit => "invalid digit",
        IntErrorKind::PosOverflow => "too large",
        IntErrorKind::NegOverflow => "too small",
        _ => "unknown",
    }
}
//endregion LexerResult

#[cfg(test)]
mod tests {
    use insta::assert_snapshot;
    use miette::{GraphicalReportHandler, GraphicalTheme, NamedSource, Report};

    use super::Lexer;
    use crate::test_utils::{dedent, fmt_snapshot_case};

    #[test]
    #[allow(clippy::approx_constant)]
    fn tokens() {
        let cases = vec![
            ("operators", "+ - * / ^ ** ="),
            ("integer", "0 42 0000123 123456789"),
            ("float", "3.14 -0.5 1.0"),
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
        ];

        let cases = cases
            .into_iter()
            .map(|(label, input)| {
                let tokens: Vec<_> = Lexer::new(input).map(Result::unwrap).collect();
                fmt_snapshot_case(
                    label,
                    &[("input", input), ("tokens", &format!("{tokens:?}"))],
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n");

        assert_snapshot!(cases);
    }

    #[test]
    fn distinguishes_assignment_and_equality() {
        let tokens: Vec<_> = Lexer::new("= ==").map(Result::unwrap).collect();
        assert_eq!(tokens, vec![super::Token::Eq, super::Token::Equal]);
    }

    #[test]
    fn lexes_colon() {
        let tokens: Vec<_> = Lexer::new(":").map(Result::unwrap).collect();
        assert_eq!(tokens, vec![super::Token::Colon]);
    }

    #[test]
    fn errors() {
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

        let cases = cases
            .into_iter()
            .map(|(label, input)| {
                let error = Lexer::new(input)
                    .next()
                    .expect("expected a lexer result")
                    .expect_err("expected a lexer error");
                let error = error.to_string();
                fmt_snapshot_case(label, &[("input", input), ("error", &error)])
            })
            .collect::<Vec<_>>()
            .join("\n\n");

        assert_snapshot!(cases);
    }

    #[test]
    fn diagnostics() {
        let cases = [
            (
                "invalid number",
                "{
                    foo: 1
                    overflow: 9223372036854775808
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

        let handler = GraphicalReportHandler::new_themed(GraphicalTheme::none());
        let cases = cases
            .into_iter()
            .map(|(label, input)| {
                let input = &dedent(input);
                let error = Lexer::new(input)
                    .find_map(Result::err)
                    .expect("expected a lexer error");
                let report = Report::new(error)
                    .with_source_code(NamedSource::new("input.dj", input.to_owned()));
                let mut rendered = String::new();
                handler
                    .render_report(&mut rendered, report.as_ref())
                    .expect("rendering a lexer error should succeed");
                fmt_snapshot_case(label, &[("input", input), ("error", &rendered)])
            })
            .collect::<Vec<_>>()
            .join("\n\n");

        assert_snapshot!(cases);
    }
}
