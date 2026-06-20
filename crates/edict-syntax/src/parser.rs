//! Recursive-descent parser for the minimal-v1 Edict surface grammar.
//!
//! Produces an [`ast::Module`]. Keywords are matched contextually by identifier
//! text so they remain usable as member names after `.`.

use crate::ast::{
    BinOp, Block, BoundRef, Decl, Expr, FieldConstraint, FieldDecl, Import, ImportKind,
    IntentClause, IntentDecl, Module, PackageRef, Param, RecordEntry, ScalarRefine, Stmt, TypeDecl,
    TypeExpr, TypeRef, UnOp,
};
use crate::token::{lex, Span, Token, TokenKind};

/// A parse failure: a message plus the source span where it was detected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub message: String,
    pub span: Span,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "parse error at {}..{}: {}",
            self.span.start, self.span.end, self.message
        )
    }
}

impl std::error::Error for ParseError {}

/// Parse Edict source into a [`Module`].
///
/// # Errors
/// Returns a [`ParseError`] on the first lexing or grammar violation.
pub fn parse_module(src: &str) -> Result<Module, ParseError> {
    let tokens = lex(src).map_err(|e| ParseError {
        message: e.message,
        span: e.span,
    })?;
    Parser::new(tokens).module()
}

struct Parser {
    tokens: Vec<Token>,
    idx: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, idx: 0 }
    }

    // --- cursor helpers ---

    fn peek(&self) -> &TokenKind {
        &self.tokens[self.idx].kind
    }

    fn peek_span(&self) -> Span {
        self.tokens[self.idx].span
    }

    fn prev_end(&self) -> usize {
        if self.idx == 0 {
            0
        } else {
            self.tokens[self.idx - 1].span.end
        }
    }

    fn at_eof(&self) -> bool {
        matches!(self.peek(), TokenKind::Eof)
    }

    fn err<T>(&self, message: impl Into<String>) -> Result<T, ParseError> {
        Err(ParseError {
            message: message.into(),
            span: self.peek_span(),
        })
    }

    /// Match a punctuation token (no payload) and consume it on success.
    fn eat(&mut self, k: &TokenKind) -> bool {
        if self.peek() == k {
            self.idx += 1;
            true
        } else {
            false
        }
    }

    fn expect(&mut self, k: &TokenKind) -> Result<(), ParseError> {
        if self.eat(k) {
            Ok(())
        } else {
            self.err(format!("expected {k:?}, found {:?}", self.peek()))
        }
    }

    fn at_kw(&self, kw: &str) -> bool {
        matches!(self.peek(), TokenKind::Ident(s) if s == kw)
    }

    fn eat_kw(&mut self, kw: &str) -> bool {
        if self.at_kw(kw) {
            self.idx += 1;
            true
        } else {
            false
        }
    }

    fn expect_kw(&mut self, kw: &str) -> Result<(), ParseError> {
        if self.eat_kw(kw) {
            Ok(())
        } else {
            self.err(format!("expected keyword `{kw}`, found {:?}", self.peek()))
        }
    }

    /// Read any bare identifier (keywords are valid here, e.g. member names).
    fn ident(&mut self) -> Result<String, ParseError> {
        match self.peek().clone() {
            TokenKind::Ident(s) => {
                self.idx += 1;
                Ok(s)
            }
            other => self.err(format!("expected identifier, found {other:?}")),
        }
    }

    fn string(&mut self) -> Result<String, ParseError> {
        match self.peek().clone() {
            TokenKind::Str(s) => {
                self.idx += 1;
                Ok(s)
            }
            other => self.err(format!("expected string literal, found {other:?}")),
        }
    }

    /// A dotted coordinate: `a.b.c`.
    fn path(&mut self) -> Result<Vec<String>, ParseError> {
        let mut parts = vec![self.ident()?];
        while *self.peek() == TokenKind::Dot {
            self.idx += 1;
            parts.push(self.ident()?);
        }
        Ok(parts)
    }

    // --- module ---

    fn module(&mut self) -> Result<Module, ParseError> {
        let package = self.package_decl()?;
        let mut imports = Vec::new();
        while self.at_kw("use") {
            imports.push(self.import()?);
        }
        let mut decls = Vec::new();
        while !self.at_eof() {
            decls.push(self.decl()?);
        }
        Ok(Module {
            package,
            imports,
            decls,
        })
    }

    fn package_ref(&mut self) -> Result<PackageRef, ParseError> {
        let start = self.peek_span().start;
        let path = self.path()?;
        self.expect(&TokenKind::At)?;
        let version = self.version()?;
        Ok(PackageRef {
            path,
            version,
            span: Span::new(start, self.prev_end()),
        })
    }

    /// A package version: starts with a digit, then a run of digit / `.` / `-` /
    /// identifier segments that are *adjacent* (no intervening whitespace).
    /// Whitespace terminates the version (SPEC grammar `version`).
    fn version(&mut self) -> Result<String, ParseError> {
        if !matches!(self.peek(), TokenKind::Int { .. }) {
            return self.err("expected package version (must start with a digit)");
        }
        let mut s = String::new();
        let mut last_end: Option<usize> = None;
        loop {
            let span = self.peek_span();
            if last_end.is_some_and(|end| span.start != end) {
                break; // whitespace gap ends the version
            }
            match self.peek().clone() {
                TokenKind::Int { value, suffix } => {
                    s.push_str(&value);
                    if let Some(suf) = suffix {
                        s.push_str(suf.lexeme());
                    }
                }
                TokenKind::Dot => s.push('.'),
                TokenKind::Minus => s.push('-'),
                TokenKind::Ident(t) => s.push_str(&t),
                _ => break,
            }
            last_end = Some(span.end);
            self.idx += 1;
        }
        Ok(s)
    }

    fn package_decl(&mut self) -> Result<PackageRef, ParseError> {
        self.expect_kw("package")?;
        let pr = self.package_ref()?;
        self.expect(&TokenKind::Semi)?;
        Ok(pr)
    }

    fn import(&mut self) -> Result<Import, ParseError> {
        let start = self.peek_span().start;
        self.expect_kw("use")?;
        let kind = if self.eat_kw("shape") {
            ImportKind::Shape
        } else if self.eat_kw("lawpack") {
            ImportKind::Lawpack
        } else if self.eat_kw("target") {
            ImportKind::Target
        } else if self.eat_kw("core") {
            ImportKind::Core
        } else if self.eat_kw("capability") {
            ImportKind::Capability
        } else {
            return self.err("expected import kind (shape|lawpack|target|core|capability)");
        };

        let (package, shape_path) = if kind == ImportKind::Shape {
            (None, Some(self.string()?))
        } else {
            (Some(self.package_ref()?), None)
        };

        let digest = if self.eat_kw("digest") {
            Some(self.string()?)
        } else {
            None
        };

        self.expect_kw("as")?;
        let alias = self.ident()?;
        self.expect(&TokenKind::Semi)?;
        Ok(Import {
            kind,
            package,
            shape_path,
            digest,
            alias,
            span: Span::new(start, self.prev_end()),
        })
    }

    fn decl(&mut self) -> Result<Decl, ParseError> {
        if self.at_kw("type") {
            Ok(Decl::Type(self.type_decl()?))
        } else if self.at_kw("intent") {
            Ok(Decl::Intent(self.intent_decl()?))
        } else {
            self.err(format!(
                "expected `type` or `intent` declaration, found {:?}",
                self.peek()
            ))
        }
    }

    // --- types ---

    fn type_decl(&mut self) -> Result<TypeDecl, ParseError> {
        let start = self.peek_span().start;
        self.expect_kw("type")?;
        let name = self.ident()?;
        let mut params = Vec::new();
        if self.eat(&TokenKind::Lt) {
            params.push(self.ident()?);
            while self.eat(&TokenKind::Comma) {
                params.push(self.ident()?);
            }
            self.expect(&TokenKind::Gt)?;
        }
        self.expect(&TokenKind::Eq)?;
        let body = if *self.peek() == TokenKind::LBrace {
            TypeExpr::Record(self.record_type()?)
        } else {
            TypeExpr::Ref(self.type_ref()?)
        };
        self.expect(&TokenKind::Semi)?;
        Ok(TypeDecl {
            name,
            params,
            body,
            span: Span::new(start, self.prev_end()),
        })
    }

    fn record_type(&mut self) -> Result<Vec<FieldDecl>, ParseError> {
        self.expect(&TokenKind::LBrace)?;
        let mut fields = Vec::new();
        while *self.peek() != TokenKind::RBrace {
            let start = self.peek_span().start;
            let name = self.ident()?;
            self.expect(&TokenKind::Colon)?;
            let ty = self.type_ref()?;
            let constraints = self.field_constraints()?;
            fields.push(FieldDecl {
                name,
                ty,
                constraints,
                span: Span::new(start, self.prev_end()),
            });
            if !self.eat(&TokenKind::Comma) {
                break;
            }
        }
        self.expect(&TokenKind::RBrace)?;
        Ok(fields)
    }

    fn field_constraints(&mut self) -> Result<Vec<FieldConstraint>, ParseError> {
        let mut out = Vec::new();
        loop {
            if self.eat_kw("max") {
                self.expect(&TokenKind::Eq)?;
                out.push(FieldConstraint::Max(self.bound_ref()?));
            } else if self.eat_kw("min") {
                self.expect(&TokenKind::Eq)?;
                out.push(FieldConstraint::Min(self.bound_ref()?));
            } else if self.eat_kw("pattern") {
                self.expect(&TokenKind::Eq)?;
                out.push(FieldConstraint::Pattern(self.string()?));
            } else if self.eat_kw("canonical") {
                self.expect(&TokenKind::Eq)?;
                out.push(FieldConstraint::Canonical(self.ident()?));
            } else {
                return Ok(out);
            }
        }
    }

    fn bound_ref(&mut self) -> Result<BoundRef, ParseError> {
        match self.peek().clone() {
            TokenKind::Int { value, .. } => {
                self.idx += 1;
                value
                    .parse::<u64>()
                    .map(BoundRef::Int)
                    .map_err(|_| ParseError {
                        message: format!("invalid integer bound `{value}`"),
                        span: self.peek_span(),
                    })
            }
            TokenKind::Ident(_) => Ok(BoundRef::Coord(self.path()?)),
            other => self.err(format!(
                "expected bound (integer or coordinate), found {other:?}"
            )),
        }
    }

    fn type_ref(&mut self) -> Result<TypeRef, ParseError> {
        let name = self.ident()?;
        match name.as_str() {
            "String" => Ok(TypeRef::StringTy(self.maybe_scalar_refine()?)),
            "Bytes" => Ok(TypeRef::BytesTy(self.maybe_bytes_refine()?)),
            "Option" => {
                self.expect(&TokenKind::Lt)?;
                let inner = self.type_ref()?;
                self.expect(&TokenKind::Gt)?;
                Ok(TypeRef::Option(Box::new(inner)))
            }
            "CapabilityRef" => {
                self.expect(&TokenKind::Lt)?;
                let inner = self.type_ref()?;
                self.expect(&TokenKind::Gt)?;
                Ok(TypeRef::CapabilityRef(Box::new(inner)))
            }
            "List" => {
                self.expect(&TokenKind::Lt)?;
                let elem = self.type_ref()?;
                self.expect(&TokenKind::Comma)?;
                self.expect_kw("max")?;
                self.expect(&TokenKind::Eq)?;
                let max = self.bound_ref()?;
                self.expect(&TokenKind::Gt)?;
                Ok(TypeRef::List {
                    elem: Box::new(elem),
                    max,
                })
            }
            "Map" => {
                self.expect(&TokenKind::Lt)?;
                let key = self.type_ref()?;
                self.expect(&TokenKind::Comma)?;
                let value = self.type_ref()?;
                self.expect(&TokenKind::Comma)?;
                self.expect_kw("max")?;
                self.expect(&TokenKind::Eq)?;
                let max = self.bound_ref()?;
                self.expect(&TokenKind::Gt)?;
                Ok(TypeRef::Map {
                    key: Box::new(key),
                    value: Box::new(value),
                    max,
                })
            }
            _ => {
                // qualified name with optional generic type-args
                let mut path = vec![name];
                while *self.peek() == TokenKind::Dot {
                    self.idx += 1;
                    path.push(self.ident()?);
                }
                let mut args = Vec::new();
                if self.eat(&TokenKind::Lt) {
                    args.push(self.type_ref()?);
                    while self.eat(&TokenKind::Comma) {
                        args.push(self.type_ref()?);
                    }
                    self.expect(&TokenKind::Gt)?;
                }
                Ok(TypeRef::Named { path, args })
            }
        }
    }

    fn maybe_scalar_refine(&mut self) -> Result<Option<ScalarRefine>, ParseError> {
        if !self.eat(&TokenKind::Lt) {
            return Ok(None);
        }
        self.expect_kw("max")?;
        self.expect(&TokenKind::Eq)?;
        let max = self.bound_ref()?;
        let canonical = if self.eat(&TokenKind::Comma) {
            self.expect_kw("canonical")?;
            self.expect(&TokenKind::Eq)?;
            Some(self.ident()?)
        } else {
            None
        };
        self.expect(&TokenKind::Gt)?;
        Ok(Some(ScalarRefine { max, canonical }))
    }

    fn maybe_bytes_refine(&mut self) -> Result<Option<u64>, ParseError> {
        if !self.eat(&TokenKind::Lt) {
            return Ok(None);
        }
        self.expect_kw("max")?;
        self.expect(&TokenKind::Eq)?;
        let max = match self.bound_ref()? {
            BoundRef::Int(n) => n,
            BoundRef::Coord(_) => {
                return self.err("Bytes max must be an integer or digest-locked bound ref");
            }
        };
        self.expect(&TokenKind::Gt)?;
        Ok(Some(max))
    }

    // --- intents ---

    fn intent_decl(&mut self) -> Result<IntentDecl, ParseError> {
        let start = self.peek_span().start;
        self.expect_kw("intent")?;
        let name = self.ident()?;
        self.expect(&TokenKind::LParen)?;
        let mut params = Vec::new();
        while *self.peek() != TokenKind::RParen {
            let pstart = self.peek_span().start;
            let pname = self.ident()?;
            self.expect(&TokenKind::Colon)?;
            let ty = self.type_ref()?;
            params.push(Param {
                name: pname,
                ty,
                span: Span::new(pstart, self.prev_end()),
            });
            if !self.eat(&TokenKind::Comma) {
                break;
            }
        }
        self.expect(&TokenKind::RParen)?;
        self.expect_kw("returns")?;
        let returns = self.type_ref()?;

        let mut clauses = Vec::new();
        loop {
            if self.at_kw("profile") {
                self.idx += 1;
                clauses.push(IntentClause::Profile(self.path()?));
            } else if self.at_kw("implements") {
                self.idx += 1;
                clauses.push(IntentClause::Implements(self.path()?));
            } else if self.at_kw("basis") {
                self.idx += 1;
                if self.eat_kw("none") {
                    clauses.push(IntentClause::Basis(None));
                } else {
                    clauses.push(IntentClause::Basis(Some(self.expr()?)));
                }
            } else if self.at_kw("footprint") {
                self.idx += 1;
                self.expect(&TokenKind::Le)?;
                clauses.push(IntentClause::Footprint(self.path()?));
            } else if self.at_kw("budget") {
                self.idx += 1;
                self.expect(&TokenKind::Le)?;
                clauses.push(IntentClause::Budget(self.path()?));
            } else if self.at_kw("where") {
                self.idx += 1;
                let mut preds = vec![self.expr()?];
                while self.eat(&TokenKind::Comma) {
                    preds.push(self.expr()?);
                }
                clauses.push(IntentClause::Where(preds));
            } else {
                break;
            }
        }

        let body = self.block()?;
        Ok(IntentDecl {
            name,
            params,
            returns,
            clauses,
            body,
            span: Span::new(start, self.prev_end()),
        })
    }

    fn block(&mut self) -> Result<Block, ParseError> {
        let start = self.peek_span().start;
        self.expect(&TokenKind::LBrace)?;
        let mut stmts = Vec::new();
        while *self.peek() != TokenKind::RBrace {
            stmts.push(self.stmt()?);
        }
        self.expect(&TokenKind::RBrace)?;
        Ok(Block {
            stmts,
            span: Span::new(start, self.prev_end()),
        })
    }

    fn stmt(&mut self) -> Result<Stmt, ParseError> {
        let start = self.peek_span().start;
        if self.eat_kw("let") {
            let name = self.ident()?;
            let ty = if self.eat(&TokenKind::Colon) {
                Some(self.type_ref()?)
            } else {
                None
            };
            self.expect(&TokenKind::Eq)?;
            let value = self.expr()?;
            self.expect(&TokenKind::Semi)?;
            Ok(Stmt::Let {
                name,
                ty,
                value,
                span: Span::new(start, self.prev_end()),
            })
        } else if self.eat_kw("return") {
            let value = self.expr()?;
            self.expect(&TokenKind::Semi)?;
            Ok(Stmt::Return {
                value,
                span: Span::new(start, self.prev_end()),
            })
        } else {
            self.err(format!(
                "expected statement (`let`/`return`), found {:?}",
                self.peek()
            ))
        }
    }

    // --- expressions (precedence climbing) ---

    fn expr(&mut self) -> Result<Expr, ParseError> {
        self.logic_or()
    }

    fn binop_left(
        &mut self,
        next: fn(&mut Self) -> Result<Expr, ParseError>,
        ops: &[(TokenKind, BinOp)],
    ) -> Result<Expr, ParseError> {
        let start = self.peek_span().start;
        let mut lhs = next(self)?;
        'outer: loop {
            for (tok, op) in ops {
                if self.peek() == tok {
                    self.idx += 1;
                    let rhs = next(self)?;
                    lhs = Expr::Binary {
                        op: *op,
                        lhs: Box::new(lhs),
                        rhs: Box::new(rhs),
                        span: Span::new(start, self.prev_end()),
                    };
                    continue 'outer;
                }
            }
            return Ok(lhs);
        }
    }

    fn logic_or(&mut self) -> Result<Expr, ParseError> {
        self.binop_left(Self::logic_and, &[(TokenKind::PipePipe, BinOp::Or)])
    }

    fn logic_and(&mut self) -> Result<Expr, ParseError> {
        self.binop_left(Self::equality, &[(TokenKind::AmpAmp, BinOp::And)])
    }

    fn equality(&mut self) -> Result<Expr, ParseError> {
        self.binop_left(
            Self::relational,
            &[(TokenKind::EqEq, BinOp::Eq), (TokenKind::Ne, BinOp::Ne)],
        )
    }

    fn relational(&mut self) -> Result<Expr, ParseError> {
        self.binop_left(
            Self::additive,
            &[
                (TokenKind::Lt, BinOp::Lt),
                (TokenKind::Le, BinOp::Le),
                (TokenKind::Gt, BinOp::Gt),
                (TokenKind::Ge, BinOp::Ge),
            ],
        )
    }

    fn additive(&mut self) -> Result<Expr, ParseError> {
        self.binop_left(
            Self::multiplicative,
            &[
                (TokenKind::Plus, BinOp::Add),
                (TokenKind::Minus, BinOp::Sub),
            ],
        )
    }

    fn multiplicative(&mut self) -> Result<Expr, ParseError> {
        self.binop_left(
            Self::unary,
            &[
                (TokenKind::Star, BinOp::Mul),
                (TokenKind::Slash, BinOp::Div),
                (TokenKind::Percent, BinOp::Rem),
            ],
        )
    }

    fn unary(&mut self) -> Result<Expr, ParseError> {
        let start = self.peek_span().start;
        let op = match self.peek() {
            TokenKind::Bang => Some(UnOp::Not),
            TokenKind::Minus => Some(UnOp::Neg),
            _ => None,
        };
        if let Some(op) = op {
            self.idx += 1;
            let operand = self.unary()?;
            Ok(Expr::Unary {
                op,
                operand: Box::new(operand),
                span: Span::new(start, self.prev_end()),
            })
        } else {
            self.postfix()
        }
    }

    fn postfix(&mut self) -> Result<Expr, ParseError> {
        let start = self.peek_span().start;
        let mut e = self.primary()?;
        while *self.peek() == TokenKind::Dot {
            self.idx += 1;
            let field = self.ident()?;
            e = Expr::Field {
                base: Box::new(e),
                field,
                span: Span::new(start, self.prev_end()),
            };
        }
        Ok(e)
    }

    fn primary(&mut self) -> Result<Expr, ParseError> {
        let span = self.peek_span();
        match self.peek().clone() {
            TokenKind::Int { value, suffix } => {
                self.idx += 1;
                Ok(Expr::Int {
                    value,
                    suffix,
                    span,
                })
            }
            TokenKind::Str(value) => {
                self.idx += 1;
                Ok(Expr::Str { value, span })
            }
            TokenKind::Ident(name) => {
                self.idx += 1;
                Ok(Expr::Ident { name, span })
            }
            TokenKind::LParen => {
                self.idx += 1;
                let e = self.expr()?;
                self.expect(&TokenKind::RParen)?;
                Ok(e)
            }
            TokenKind::LBrace => self.record_literal(),
            other => self.err(format!("expected expression, found {other:?}")),
        }
    }

    fn record_literal(&mut self) -> Result<Expr, ParseError> {
        let start = self.peek_span().start;
        self.expect(&TokenKind::LBrace)?;
        let mut entries = Vec::new();
        while *self.peek() != TokenKind::RBrace {
            if self.eat(&TokenKind::Spread) {
                entries.push(RecordEntry::Spread(self.expr()?));
            } else {
                let espan = self.peek_span();
                let name = self.ident()?;
                if self.eat(&TokenKind::Colon) {
                    entries.push(RecordEntry::Field {
                        name,
                        value: self.expr()?,
                    });
                } else {
                    entries.push(RecordEntry::Shorthand { name, span: espan });
                }
            }
            if !self.eat(&TokenKind::Comma) {
                break;
            }
        }
        self.expect(&TokenKind::RBrace)?;
        Ok(Expr::Record {
            entries,
            span: Span::new(start, self.prev_end()),
        })
    }
}
