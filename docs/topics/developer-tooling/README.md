# Developer Tooling

The developer-tooling alpha starts with editor-facing source highlighting. The
public contract in this branch is `edict_syntax::highlight_source`, which
classifies source spans into stable `HighlightRole` values for editor adapters.

Highlighting is intentionally lexical. It does not parse, resolve, type-check,
lower to Core, or evaluate admission policy. It keeps comments visible to
editors even though the parser treats comments as trivia and excludes them from
semantic identity.

Supported roles are:

- `Comment`
- `Identifier`
- `Keyword`
- `Number`
- `Operator`
- `Punctuation`
- `String`
- `TypeIdentifier`

Tree-sitter, TextMate, VS Code, Vim, Zed, and jedit integration artifacts remain
future `v0.6.0-alpha.1` work. This shelf owns the behavior those adapters must
preserve.
