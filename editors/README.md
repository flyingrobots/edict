# Edict Editor Integrations

This directory contains first-party editor integration artifacts for Edict.

## Supported In This Repository

- `vscode/`: a thin VS Code/Cursor extension package that registers `.edict`
  files and uses the canonical TextMate grammar for syntax highlighting.

## Grammar Artifacts

- `../grammars/textmate/edict.tmLanguage.json` is the canonical TextMate grammar
  source.
- `../grammars/tree-sitter-edict/` contains the Tree-sitter grammar source,
  generated parser source, highlight query, and current-subset corpus.

## Current Limits

The VS Code package is source-controlled and locally installable, but this
repository does not publish a Marketplace package yet. Vim, Zed, and jedit
extension packages remain future work; those consumers should use the grammar
artifacts directly until first-party packages land.
