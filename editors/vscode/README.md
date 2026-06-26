# Edict VS Code Extension

This package registers `.edict` files for VS Code-compatible editors and maps
them to the Edict TextMate grammar.

## Local Install

From this directory:

```text
npx @vscode/vsce package
code --install-extension edict-vscode-0.6.0-alpha.1.vsix
```

Cursor can install the same VSIX package from its extensions view.

## Scope

The extension only provides language registration and lexical syntax
highlighting. It does not ship a language server, compiler command, formatter,
semantic tokens, diagnostics, or Tree-sitter runtime integration.
