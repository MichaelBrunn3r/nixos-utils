use std::fmt::{self, Write};

use super::{AST, BinaryOp, Expr, Identifier, Statement, UnaryOp};

pub struct ASTPretty<'ast, 'config, 'input> {
    ast: &'ast AST<'input>,
    config: &'config ASTPrettyConfig,
}

impl fmt::Display for ASTPretty<'_, '_, '_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.ast
            .pretty_doc(self.config)
            .render(formatter, self.config, 0)
    }
}

#[derive(Debug, Clone)]
pub struct ASTPrettyConfig {
    pub page_width: usize,
    pub indent_width: usize,
}

impl Default for ASTPrettyConfig {
    fn default() -> Self {
        Self {
            page_width: 80,
            indent_width: 4,
        }
    }
}

impl<'input> AST<'input> {
    #[must_use]
    pub const fn pretty<'ast, 'config>(
        &'ast self,
        config: &'config ASTPrettyConfig,
    ) -> ASTPretty<'ast, 'config, 'input> {
        ASTPretty { ast: self, config }
    }

    #[must_use]
    pub fn pretty_string(&self) -> String {
        self.pretty(&ASTPrettyConfig::default()).to_string()
    }
}

impl AST<'_> {
    fn pretty_doc(&self, config: &ASTPrettyConfig) -> Doc {
        if self
            .statements
            .iter()
            .all(|statement| matches!(statement, Statement::KV(_)))
        {
            let entries = join(
                self.statements
                    .iter()
                    .map(|statement| statement.pretty_doc(config)),
                &concat([text(","), Doc::Line]),
            );
            group(concat([
                text("map{"),
                nest(config.indent_width, concat([Doc::SoftLine, entries])),
                Doc::SoftLine,
                text("}"),
            ]))
        } else {
            join(
                self.statements
                    .iter()
                    .map(|statement| statement.pretty_doc(config)),
                &Doc::HardLine,
            )
        }
    }
}

impl Expr<'_> {
    fn pretty_doc(&self, config: &ASTPrettyConfig) -> Doc {
        match self {
            Self::Bool(value) => text(value.to_string()),
            Self::Int(value) => text(value.to_string()),
            Self::Float(value) => text(value.to_string()),
            Self::Str(value) => text(format!("{value:?}")),
            Self::Id(identifier) => identifier_document(identifier),
            Self::Unary { op, value } => {
                let operator = match op {
                    UnaryOp::Positive => "Positive",
                    UnaryOp::Negative => "Negative",
                };
                concat([
                    text(format!("{operator}(")),
                    value.pretty_doc(config),
                    text(")"),
                ])
            }
            Self::Binary { left, op, right } => {
                let operator = match op {
                    BinaryOp::Add => "Add",
                    BinaryOp::Sub => "Sub",
                    BinaryOp::Mul => "Mul",
                    BinaryOp::Div => "Div",
                    BinaryOp::Exp => "Exp",
                };
                concat([
                    text(format!("{operator}(")),
                    left.pretty_doc(config),
                    text(", "),
                    right.pretty_doc(config),
                    text(")"),
                ])
            }
            Self::Access { object, name } => concat([
                text("Access("),
                object.pretty_doc(config),
                text(", "),
                text(*name),
                text(")"),
            ]),
            Self::Call { callee, arguments } => {
                let arguments = join(
                    arguments.iter().map(|argument| argument.pretty_doc(config)),
                    &concat([text(","), Doc::Line]),
                );
                group(concat([
                    text("Call("),
                    callee.pretty_doc(config),
                    text(", ["),
                    nest(config.indent_width, concat([Doc::SoftLine, arguments])),
                    Doc::SoftLine,
                    text("])"),
                ]))
            }
        }
    }
}

impl Statement<'_> {
    fn pretty_doc(&self, config: &ASTPrettyConfig) -> Doc {
        match self {
            Self::Expr(expr) => concat([text("Expr("), expr.pretty_doc(config), text(")")]),
            Self::KV(pair) => concat([
                key_document(pair.key),
                text(" = "),
                pair.expr.pretty_doc(config),
            ]),
            Self::Use(use_statement) => concat([
                text("Use("),
                path_document(&use_statement.path),
                if use_statement.wildcard {
                    text(".*")
                } else {
                    Doc::Nil
                },
                text(")"),
            ]),
        }
    }
}

#[derive(Clone, Copy)]
enum Mode {
    Flat,
    Break,
}

#[derive(Clone)]
enum Doc {
    Nil,
    Text(String),
    Line,
    SoftLine,
    HardLine,
    Concat(Vec<Self>),
    Nest(usize, Box<Self>),
    Group(Box<Self>),
}

impl Doc {
    fn render<W: Write>(
        self,
        writer: &mut W,
        config: &ASTPrettyConfig,
        initial_indent: usize,
    ) -> fmt::Result {
        let mut stack = vec![(initial_indent, Mode::Break, self)];
        let mut column = initial_indent;

        while let Some((indent, mode, doc)) = stack.pop() {
            match doc {
                Self::Nil => {}
                Self::Text(value) => {
                    column += value.len();
                    writer.write_str(&value)?;
                }
                Self::Line => match mode {
                    Mode::Flat => {
                        writer.write_char(' ')?;
                        column += 1;
                    }
                    Mode::Break => {
                        writer.write_char('\n')?;
                        write_indentation(writer, indent)?;
                        column = indent;
                    }
                },
                Self::SoftLine => {
                    if matches!(mode, Mode::Break) {
                        writer.write_char('\n')?;
                        write_indentation(writer, indent)?;
                        column = indent;
                    }
                }
                Self::HardLine => {
                    writer.write_char('\n')?;
                    write_indentation(writer, indent)?;
                    column = indent;
                }
                Self::Concat(documents) => {
                    for doc in documents.into_iter().rev() {
                        stack.push((indent, mode, doc));
                    }
                }
                Self::Nest(amount, doc) => {
                    stack.push((indent + amount, mode, *doc));
                }
                Self::Group(doc) => {
                    let flat = flatten(&doc);
                    if fits(
                        config.page_width.saturating_sub(column),
                        &stack,
                        (indent, flat.clone()),
                    ) {
                        stack.push((indent, Mode::Flat, flat));
                    } else {
                        stack.push((indent, Mode::Break, *doc));
                    }
                }
            }
        }

        Ok(())
    }
}

fn text(value: impl Into<String>) -> Doc {
    Doc::Text(value.into())
}

fn concat(documents: impl IntoIterator<Item = Doc>) -> Doc {
    let documents = documents.into_iter().collect::<Vec<_>>();
    match documents.as_slice() {
        [] => Doc::Nil,
        [_] => documents
            .into_iter()
            .next()
            .unwrap_or_else(|| unreachable!()),
        _ => Doc::Concat(documents),
    }
}

fn join(documents: impl IntoIterator<Item = Doc>, separator: &Doc) -> Doc {
    let mut result = Vec::new();
    for (index, document) in documents.into_iter().enumerate() {
        if index > 0 {
            result.push(separator.clone());
        }
        result.push(document);
    }
    concat(result)
}

fn nest(amount: usize, document: Doc) -> Doc {
    Doc::Nest(amount, Box::new(document))
}

fn group(document: Doc) -> Doc {
    Doc::Group(Box::new(document))
}

fn flatten(document: &Doc) -> Doc {
    match document {
        Doc::Nil => Doc::Nil,
        Doc::Text(value) => text(value.clone()),
        Doc::Line => text(" "),
        Doc::SoftLine | Doc::HardLine => text(""),
        Doc::Concat(documents) => concat(documents.iter().map(flatten)),
        Doc::Nest(amount, document) => nest(*amount, flatten(document)),
        Doc::Group(document) => flatten(document),
    }
}

fn fits(mut remaining: usize, stack: &[(usize, Mode, Doc)], first: (usize, Doc)) -> bool {
    let mut stack = stack.to_vec();
    stack.push((first.0, Mode::Flat, first.1));

    while let Some((indent, mode, document)) = stack.pop() {
        match document {
            Doc::Nil => {}
            Doc::Text(value) => {
                if value.len() > remaining {
                    return false;
                }
                remaining -= value.len();
            }
            Doc::Line | Doc::SoftLine | Doc::HardLine => {
                if matches!(mode, Mode::Break) {
                    return true;
                }
                remaining = remaining.saturating_sub(1);
            }
            Doc::Concat(documents) => {
                for document in documents.into_iter().rev() {
                    stack.push((indent, mode, document));
                }
            }
            Doc::Nest(amount, document) => stack.push((indent + amount, mode, *document)),
            Doc::Group(document) => stack.push((indent, mode, *document)),
        }
    }

    true
}

fn key_document(key: &str) -> Doc {
    if key
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || character == '_')
    {
        text(key)
    } else {
        text(format!("{key:?}"))
    }
}

fn write_indentation<W: Write>(writer: &mut W, indent: usize) -> fmt::Result {
    for _ in 0..indent {
        writer.write_char(' ')?;
    }
    Ok(())
}

fn identifier_document(identifier: &Identifier<'_>) -> Doc {
    match identifier {
        Identifier::Simple(value) => text(*value),
        Identifier::Qualified(path) => path_document(path),
    }
}

fn path_document(path: &[&str]) -> Doc {
    join(path.iter().map(|part| text(*part)), &text("."))
}
