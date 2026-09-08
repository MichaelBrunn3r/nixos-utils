# DJSON for VS Code

Syntax highlighting and editor support for the DJSON language.

## Features

- Highlights `.djson` and `.dj` files.
- Recognizes comments, strings, numbers, booleans, keywords, fields, operators, calls, and member access.
- Provides matching brackets and automatic quote and bracket completion.

## Development

Open the repository in VS Code and press `F5` to launch an Extension Development Host. Open a DJSON file in that window to inspect the grammar.

## Packaging

Run the repository recipe below to create a local VSIX:

```text
just vscode-djson
```
