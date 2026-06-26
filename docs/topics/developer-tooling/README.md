# Developer Tooling

The developer-tooling alpha now has three editor-facing surfaces:

- `edict_syntax::highlight_source`, which classifies source spans into stable
  `HighlightRole` values for editor adapters.
- [Tree-sitter grammar source](../../../grammars/tree-sitter-edict/grammar.js),
  generated parser source, a
  [highlight query](../../../grammars/tree-sitter-edict/queries/highlights.scm),
  and a
  [current-subset corpus](../../../grammars/tree-sitter-edict/test/corpus/current-subset.txt)
  for the accepted fixture families.
- [TextMate grammar](../../../grammars/textmate/edict.tmLanguage.json) for
  `.edict` lexical scopes in TextMate-compatible editors.

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

The Tree-sitter grammar is an editor syntax-tree artifact for the current
accepted source subset. Its corpus covers bounded hello, branch-yield effects,
read obstruction handling, and enum/variant match syntax, and the corpus source
examples must keep parsing through `edict_syntax::parse_module`.

The TextMate grammar is a lexical artifact for `.edict` files. It exposes scopes
for comments, strings, numbers, keywords, type identifiers, operators,
punctuation, and bare identifiers, aligned with the public highlighter roles.

Generated npm packages, VS Code, Vim, Zed, and jedit integration packages remain
future `v0.6.0-alpha.1` work. This shelf owns the behavior those adapters must
preserve.
