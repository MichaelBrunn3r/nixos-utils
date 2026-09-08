#![allow(clippy::cast_precision_loss)]

use crate::lexer::token::Token;
pub mod token;

pub struct Lexer<'input> {
    input: &'input str,
    pos: usize,
    curr_line: usize,
    buffer: String,
}

impl<'input> Lexer<'input> {
    #[must_use]
    pub const fn new(input: &'input str) -> Self {
        Self {
            input,
            pos: 0,
            curr_line: 0,
            buffer: String::new(),
        }
    }
}

impl<'input> Iterator for Lexer<'input> {
    type Item = LexResult<'input>;

    fn next(&mut self) -> Option<Self::Item> {
        let line_changed = self.skip_ignored();
        if line_changed {
            return Some(Ok(Token::Sep));
        }

        let c = self.peek_char()?;

        let token = match c {
            ',' | '\n' => Ok(Token::Sep),
            '(' => Ok(Token::LParen),
            ')' => Ok(Token::RParen),
            '[' => Ok(Token::LBracket),
            ']' => Ok(Token::RBracket),
            '{' => Ok(Token::LBrace),
            '}' => Ok(Token::RBrace),
            '.' => Ok(Token::Dot),
            '+' => Ok(Token::Add),
            '-' => Ok(Token::Sub),
            '^' => Ok(Token::Exp),
            '*' => {
                if self.starts_with("**") {
                    self.next_char();
                    Ok(Token::Exp)
                } else {
                    Ok(Token::Mul)
                }
            }
            '/' => Ok(Token::Div),
            ':' => Ok(Token::Colon),
            '=' => {
                if self.starts_with("==") {
                    self.next_char();
                    Ok(Token::Equal)
                } else {
                    Ok(Token::Eq)
                }
            }
            '"' | '\'' => return Some(self.read_string(c)),
            '0'..='9' => return Some(self.read_number()),
            _ => return Some(Ok(self.read_identifier())),
        };

        self.next_char();
        Some(token)
    }
}

impl<'input> Lexer<'input> {
    //region Lookahead helpers
    fn peek_char(&self) -> Option<char> {
        self.input[self.pos..].chars().next()
    }

    fn starts_with(&self, pattern: &str) -> bool {
        self.input[self.pos..].starts_with(pattern)
    }

    fn expect_peek(
        &self,
        predicate: impl FnOnce(char) -> bool,
        message: &str,
    ) -> Result<(), LexerError> {
        if self.peek_char().is_some_and(predicate) {
            Ok(())
        } else {
            Err(LexerError {
                line: self.curr_line,
                message: message.to_owned(),
            })
        }
    }
    //endregion Lookahead helpers

    fn next_char(&mut self) -> Option<char> {
        let character = self.peek_char()?;
        self.pos += character.len_utf8();
        if character == '\n' {
            self.curr_line += 1;
        }
        Some(character)
    }

    //region Skip helpers
    fn skip_ignored(&mut self) -> bool {
        let mut line_changed = false;

        loop {
            self.skip_whitespace();
            if self.starts_with("//") {
                self.skip_line_comment();
                return line_changed;
            } else if self.starts_with("/*") {
                line_changed |= self.skip_block_comment();
            } else {
                return line_changed;
            }
        }
    }

    fn skip_whitespace(&mut self) {
        while self
            .peek_char()
            .is_some_and(|character| character.is_whitespace() && character != '\n')
        {
            self.next_char();
        }
    }

    fn skip_line_comment(&mut self) {
        while self.peek_char().is_some_and(|character| character != '\n') {
            self.next_char();
        }
    }

    fn skip_block_comment(&mut self) -> bool {
        let mut line_changed = false;

        self.next_char();
        self.next_char();

        while self.peek_char().is_some() {
            if self.starts_with("*/") {
                self.next_char();
                self.next_char();
                return line_changed;
            }
            line_changed |= self.peek_char() == Some('\n');
            self.next_char();
        }

        line_changed
    }
    //endregion Skip helpers

    fn read_string(&mut self, delimiter: char) -> LexResult<'input> {
        self.next_char();
        let start = self.pos;

        while let Some(c) = self.peek_char() {
            if c == delimiter {
                let end = self.pos;
                self.next_char();
                return Ok(Token::Str(&self.input[start..end]));
            }
            if c == '\\' {
                self.next_char();
                self.next_char();
                continue;
            }
            self.next_char();
        }

        Err(LexerError {
            line: self.curr_line,
            message: "unterminated string".to_owned(),
        })
    }

    fn read_number(&mut self) -> LexResult<'input> {
        let start = self.pos;
        let mut has_decimals = false;

        let literal = loop {
            let Some(character) = self.peek_char() else {
                break &self.input[start..self.pos];
            };

            match character {
                '0'..='9' => {
                    self.next_char();
                }
                '.' if self.input[self.pos..]
                    .chars()
                    .nth(1)
                    .is_some_and(|character| {
                        character.is_ascii_digit() || matches!(character, '_' | '\'')
                    }) =>
                {
                    if has_decimals {
                        return Err(LexerError {
                            line: self.curr_line,
                            message: "invalid number".to_owned(),
                        });
                    }

                    has_decimals = true;
                    self.next_char();
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
                .map_err(|_| LexerError {
                    line: self.curr_line,
                    message: "invalid float".to_owned(),
                })
        } else {
            literal
                .parse::<i64>()
                .map(Token::Int)
                .map_err(|_| LexerError {
                    line: self.curr_line,
                    message: "invalid integer".to_owned(),
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

        while let Some(character) = self.peek_char() {
            match character {
                '0'..='9' => {
                    self.buffer.push(character);
                    self.next_char();
                }
                '.' => {
                    if has_decimals {
                        return Err(LexerError {
                            line: self.curr_line,
                            message: "invalid number".to_owned(),
                        });
                    }

                    has_decimals = true;
                    self.buffer.push(character);
                    self.next_char();
                }
                '_' | '\'' => {
                    self.next_char();
                    self.expect_peek(|next| next.is_ascii_digit(), "invalid number separator")?;
                }
                _ => break,
            }
        }

        Ok((has_decimals, self.buffer.as_str()))
    }

    fn read_identifier(&mut self) -> Token<'input> {
        let start = self.pos;

        while self.peek_char().is_some_and(|character| {
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
            self.next_char();
        }

        let identifier = &self.input[start..self.pos];
        match identifier {
            "true" => Token::Bool(true),
            "false" => Token::Bool(false),
            _ => Token::Id(identifier),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct LexerError {
    pub line: usize,
    pub message: String,
}

pub type LexResult<'input> = Result<Token<'input>, LexerError>;

#[cfg(test)]
mod tests {
    use insta::assert_snapshot;

    use super::Lexer;

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
                format!("{label}\ninput: `{input}`\ntokens: {tokens:?}")
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
            ("integer overflow", "9223372036854775808"),
            ("trailing num separator", "100_"),
            ("num separator sequence", "1__000"),
            ("heterogenous num separator sequence", "1_'000"),
            ("unterminated string", "\"unterminated"),
        ];

        let cases = cases
            .into_iter()
            .map(|(label, input)| {
                let error = Lexer::new(input)
                    .next()
                    .expect("expected a lexer result")
                    .expect_err("expected a lexer error");
                format!("{label}\ninput: `{input}`\nerror: {}", error.message)
            })
            .collect::<Vec<_>>()
            .join("\n\n");

        assert_snapshot!(cases);
    }
}
