import oniguruma from "vscode-oniguruma";
import textmate from "vscode-textmate";
import type { IGrammar, IRawGrammar } from "vscode-textmate";
import { grammar } from "../src/grammar.ts";

const wasm = await Deno.readFile(
    new URL(
        "../node_modules/vscode-oniguruma/release/onig.wasm",
        import.meta.url,
    ),
);
await oniguruma.loadWASM(wasm.buffer);

const registry = new textmate.Registry({
    onigLib: Promise.resolve({
        createOnigScanner(patterns: string[]) {
            return oniguruma.createOnigScanner(patterns);
        },
        createOnigString(input: string) {
            return oniguruma.createOnigString(input);
        },
    }),
    loadGrammar(scopeName: string): Promise<IRawGrammar | null> {
        return Promise.resolve(
            scopeName === grammar.scopeName ? grammar as unknown as IRawGrammar : null,
        );
    },
});

const djsonGrammar = await registry.loadGrammar(grammar.scopeName) as IGrammar;

function tokenize(document: string): string[][] {
    let state: Parameters<IGrammar["tokenizeLine"]>[1] = null;
    return document.split("\n").flatMap((line) => {
        const result = djsonGrammar.tokenizeLine(line, state);
        state = result.ruleStack;
        const lineTokens = result.tokens;
        return lineTokens.map((
            token,
        ) => [
            line.slice(token.startIndex, token.endIndex),
            token.scopes.join(" "),
        ]);
    });
}

const cases: [string, string, string[][]][] = [
    ["let is a keyword or map key", "let x = 1\nlet: x", [
        ["let", "keyword.control"],
        ["x", "variable.other"],
        ["=", "keyword.operator"],
        ["1", "constant.numeric"],
        ["let", "support.type.property-name"],
        [":", "punctuation.separator.key-value"],
        ["x", "variable.other"],
    ]],
    ["if and else are keywords or map keys", "if true 1 else 2\nif: 1\nelse: 2", [
        ["if", "keyword.control"],
        ["true", "constant.language.boolean"],
        ["1", "constant.numeric"],
        ["else", "keyword.control"],
        ["2", "constant.numeric"],
        ["if", "support.type.property-name"],
        [":", "punctuation.separator.key-value"],
        ["1", "constant.numeric"],
        ["else", "support.type.property-name"],
        [":", "punctuation.separator.key-value"],
        ["2", "constant.numeric"],
    ]],
    ["braces have punctuation scopes", "if true { 1 } else { 2 }", [
        ["if", "keyword.control"],
        ["true", "constant.language.boolean"],
        ["{", "punctuation.definition.block.begin"],
        ["1", "constant.numeric"],
        ["}", "punctuation.definition.block.end"],
        ["else", "keyword.control"],
        ["{", "punctuation.definition.block.begin"],
        ["2", "constant.numeric"],
        ["}", "punctuation.definition.block.end"],
    ]],
    ["hyphenated keys do not contain keywords", "if-else: 1", [
        ["if", "variable.other"],
        ["-", "keyword.operator"],
        ["else", "support.type.property-name"],
        [":", "punctuation.separator.key-value"],
        ["1", "constant.numeric"],
    ]],
    [
        "integers support signs, leading zeros, and digit separators",
        "0 00000 000001 123 1_000 1'000 -1 -1_000 -1'000",
        [
            ["0", "constant.numeric"],
            ["00000", "constant.numeric"],
            ["000001", "constant.numeric"],
            ["123", "constant.numeric"],
            ["1_000", "constant.numeric"],
            ["1'000", "constant.numeric"],
            ["-", "keyword.operator"],
            ["1", "constant.numeric"],
            ["-", "keyword.operator"],
            ["1_000", "constant.numeric"],
            ["-", "keyword.operator"],
            ["1'000", "constant.numeric"],
        ],
    ],
    [
        "floats support signs, leading zeros, and digit separators",
        "0.0 00000.1 00000.00001 123.456 1_000.0 1'000.0 1.0_5 1.0'5 -1.0 -1_000.0 -1'000.0",
        [
            ["0.0", "constant.numeric"],
            ["00000.1", "constant.numeric"],
            ["00000.00001", "constant.numeric"],
            ["123.456", "constant.numeric"],
            ["1_000.0", "constant.numeric"],
            ["1'000.0", "constant.numeric"],
            ["1.0_5", "constant.numeric"],
            ["1.0'5", "constant.numeric"],
            ["-", "keyword.operator"],
            ["1.0", "constant.numeric"],
            ["-", "keyword.operator"],
            ["1_000.0", "constant.numeric"],
            ["-", "keyword.operator"],
            ["1'000.0", "constant.numeric"],
        ],
    ],
    ["none aliases are recognized", "none null nil", [
        ["none", "constant.language.null"],
        ["null", "constant.language.null"],
        ["nil", "constant.language.null"],
    ]],
    ["quoted string delimiters are separate tokens", "\"hello\" 'world'", [
        ['"', "string.quoted.double punctuation.definition.string.begin"],
        ["hello", "string.quoted.double"],
        ['"', "string.quoted.double punctuation.definition.string.end"],
        ["'", "string.quoted.single punctuation.definition.string.begin"],
        ["world", "string.quoted.single"],
        ["'", "string.quoted.single punctuation.definition.string.end"],
    ]],
    ["quote escapes scope only their backslashes", String.raw`"a\"b" 'c\'d'`, [
        ['"', "string.quoted.double punctuation.definition.string.begin"],
        ["a", "string.quoted.double"],
        [
            "\\",
            "string.quoted.double constant.character.escape punctuation.definition.character.escape",
        ],
        ['"', "string.quoted.double constant.character.escape"],
        ["b", "string.quoted.double"],
        ['"', "string.quoted.double punctuation.definition.string.end"],
        ["'", "string.quoted.single punctuation.definition.string.begin"],
        ["c", "string.quoted.single"],
        [
            "\\",
            "string.quoted.single constant.character.escape punctuation.definition.character.escape",
        ],
        ["'", "string.quoted.single constant.character.escape"],
        ["d", "string.quoted.single"],
        ["'", "string.quoted.single punctuation.definition.string.end"],
    ]],
    [
        "whitespace escapes stay whole and backslashes split",
        String.raw`"whitespace\t\n\r" "\\x"`,
        [
            ['"', "string.quoted.double punctuation.definition.string.begin"],
            ["whitespace", "string.quoted.double"],
            [
                String.raw`\t`,
                "string.quoted.double constant.character.escape constant.character.whitespace",
            ],
            [
                String.raw`\n`,
                "string.quoted.double constant.character.escape constant.character.whitespace",
            ],
            [
                String.raw`\r`,
                "string.quoted.double constant.character.escape constant.character.whitespace",
            ],
            ['"', "string.quoted.double punctuation.definition.string.end"],
            ['"', "string.quoted.double punctuation.definition.string.begin"],
            [
                "\\",
                "string.quoted.double constant.character.escape punctuation.definition.character.escape",
            ],
            ["\\", "string.quoted.double constant.character.escape"],
            ["x", "string.quoted.double"],
            ['"', "string.quoted.double punctuation.definition.string.end"],
        ],
    ],
    ["backslashes alternate escape punctuation", `"${"\\".repeat(6)}"`, [
        ['"', "string.quoted.double punctuation.definition.string.begin"],
        [
            "\\",
            "string.quoted.double constant.character.escape punctuation.definition.character.escape",
        ],
        ["\\", "string.quoted.double constant.character.escape"],
        [
            "\\",
            "string.quoted.double constant.character.escape punctuation.definition.character.escape",
        ],
        ["\\", "string.quoted.double constant.character.escape"],
        [
            "\\",
            "string.quoted.double constant.character.escape punctuation.definition.character.escape",
        ],
        ["\\", "string.quoted.double constant.character.escape"],
        ['"', "string.quoted.double punctuation.definition.string.end"],
    ]],
    ["member access and separators have punctuation scopes", '"héllo".,', [
        ['"', "string.quoted.double punctuation.definition.string.begin"],
        ["héllo", "string.quoted.double"],
        ['"', "string.quoted.double punctuation.definition.string.end"],
        [".", "punctuation.accessor"],
        [",", "punctuation.separator"],
    ]],
    ["array delimiters have punctuation scopes", "[1]", [
        ["[", "punctuation.definition.array.begin"],
        ["1", "constant.numeric"],
        ["]", "punctuation.definition.array.end"],
    ]],
    ["parentheses have punctuation scopes", "(-1).abs()", [
        ["(", "punctuation.definition.parenthesis.begin"],
        ["-", "keyword.operator"],
        ["1", "constant.numeric"],
        [")", "punctuation.definition.parenthesis.end"],
        [".", "punctuation.accessor"],
        ["abs", "entity.name.function"],
        ["(", "punctuation.definition.parenthesis.begin"],
        [")", "punctuation.definition.parenthesis.end"],
    ]],
    [
        "hash comments cover shebangs and trailing comments",
        "#!/usr/bin/env djson\nfoo#bar\n1 # trailing",
        [
            ["#!/usr/bin/env djson", "comment.line.number-sign"],
            ["foo", "variable.other"],
            ["#bar", "comment.line.number-sign"],
            ["1", "constant.numeric"],
            ["# trailing", "comment.line.number-sign"],
        ],
    ],
    ["hash inside strings is not a comment", "\"a # b\" 'c # d'", [
        ['"', "string.quoted.double punctuation.definition.string.begin"],
        ["a # b", "string.quoted.double"],
        ['"', "string.quoted.double punctuation.definition.string.end"],
        ["'", "string.quoted.single punctuation.definition.string.begin"],
        ["c # d", "string.quoted.single"],
        ["'", "string.quoted.single punctuation.definition.string.end"],
    ]],
];

for (const [label, document, expected] of cases) {
    Deno.test(label, () => {
        const actual = tokenize(document).filter(([, scopes]) => scopes !== "source.djson");
        const expectedWithRootScope = expected.map(([text, scopes]) => [
            text,
            `source.djson ${scopes}`,
        ]);
        if (JSON.stringify(actual) !== JSON.stringify(expectedWithRootScope)) {
            throw new Error(
                `Expected tokens ${JSON.stringify(expectedWithRootScope)}, got ${
                    JSON.stringify(actual)
                }`,
            );
        }
    });
}
