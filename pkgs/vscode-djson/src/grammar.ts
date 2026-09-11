const comments = {
    patterns: [
        { name: "comment.line.double-slash", match: "//.*$" },
        { name: "comment.line.number-sign", match: "#.*$" },
    ],
} as const;

const escapePatterns = [
    {
        name: "constant.character.escape",
        match: "(\\\\)[nrt]",
        captures: {
            0: { name: "constant.character.whitespace" },
        },
    },
    {
        name: "constant.character.escape",
        match: "(\\\\)(.)",
        captures: {
            1: { name: "punctuation.definition.character.escape" },
        },
    },
] as const;

const strings = {
    patterns: [
        {
            name: "string.quoted.double",
            begin: '"',
            end: '"',
            applyEndPatternLast: true,
            beginCaptures: {
                0: { name: "punctuation.definition.string.begin" },
            },
            endCaptures: {
                0: { name: "punctuation.definition.string.end" },
            },
            patterns: escapePatterns,
        },
        {
            name: "string.quoted.single",
            begin: "'",
            end: "'",
            applyEndPatternLast: true,
            beginCaptures: {
                0: { name: "punctuation.definition.string.begin" },
            },
            endCaptures: {
                0: { name: "punctuation.definition.string.end" },
            },
            patterns: escapePatterns,
        },
    ],
} as const;

const numbers = {
    patterns: [
        {
            name: "constant.numeric",
            match: "\\b-?\\d(?:\\d|[_']\\d)*(?:\\.\\d(?:\\d|[_']\\d)*)?\\b",
        },
    ],
} as const;

const booleans = {
    patterns: [{
        name: "constant.language.boolean",
        match: "\\b(?:true|false)\\b",
    }],
} as const;

const none = {
    patterns: [{
        name: "constant.language.null",
        match: "\\b(?:none|null|nil)\\b",
    }],
} as const;

const keywords = {
    patterns: [{ name: "keyword.control", match: "\\blet\\b(?!\\s*:)" }],
} as const;

const functions = {
    patterns: [
        {
            name: "entity.name.function",
            match: "\\b[A-Za-z_][A-Za-z0-9_]*(?=\\s*\\()",
        },
    ],
} as const;

const operators = {
    patterns: [
        {
            name: "keyword.operator",
            match: "(?:==|!=|<=|>=|&&|\\|\\||[+*/^<>=-])",
        },
    ],
} as const;

const fields = {
    patterns: [
        {
            name: "support.type.property-name",
            match: "\\b[A-Za-z_][A-Za-z0-9_]*(?=\\s*:)",
        },
    ],
} as const;

const variables = {
    patterns: [{
        name: "variable.other",
        match: "\\b[A-Za-z_][A-Za-z0-9_]*\\b",
    }],
} as const;

export const grammar = {
    $schema: "https://raw.githubusercontent.com/martinring/tmlanguage/master/tmlanguage.json",
    name: "DJSON",
    scopeName: "source.djson",
    fileTypes: ["dj", "djson"],
    patterns: [
        ...comments.patterns,
        ...strings.patterns,
        ...numbers.patterns,
        ...booleans.patterns,
        ...none.patterns,
        ...keywords.patterns,
        ...functions.patterns,
        { name: "punctuation.accessor", match: "\\." },
        { name: "punctuation.separator", match: "," },
        { name: "punctuation.separator.key-value", match: ":" },
        { name: "punctuation.definition.array.begin", match: "\\[" },
        { name: "punctuation.definition.array.end", match: "\\]" },
        { name: "punctuation.definition.parenthesis.begin", match: "\\(" },
        { name: "punctuation.definition.parenthesis.end", match: "\\)" },
        ...operators.patterns,
        ...fields.patterns,
        ...variables.patterns,
    ],
} as const;
