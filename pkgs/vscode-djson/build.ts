import { grammar } from "./src/grammar.ts";

await Deno.mkdir("syntaxes", { recursive: true });
await Deno.writeTextFile(
    "syntaxes/djson.tmLanguage.json",
    `${JSON.stringify(grammar, null, "    ")}\n`,
);
