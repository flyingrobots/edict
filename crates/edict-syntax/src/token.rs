//! Edict lexical tokens and the hand-written lexer.
//!
//! Whitespace and comments are not semantic (SPEC - Edict Language v1, Lexical
//! Rules). Identifiers cover every bare word — including clause/declaration
//! keywords — so the parser can treat keywords contextually (they "may appear
//! after `.` as member names").

use std::fmt;

/// A half-open byte span `[start, end)` into the source, retained for
/// diagnostics but excluded from canonical Core IR.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    #[must_use]
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }
}

/// Typed-integer-literal suffix (`1u64`, `64_000i64`). Integer width and
/// signedness are hash-significant (I-010); the suffix is one of the two ways a
/// literal resolves its type (`EDICT-LANG-INTLIT-001`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntSuffix {
    I32,
    I64,
    U32,
    U64,
}

impl IntSuffix {
    fn from_str(s: &str) -> Option<Self> {
        match s {
            "i32" => Some(Self::I32),
            "i64" => Some(Self::I64),
            "u32" => Some(Self::U32),
            "u64" => Some(Self::U64),
            _ => None,
        }
    }
}

/// The lexical category of a token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenKind {
    /// Any bare word `[A-Za-z_][A-Za-z0-9_]*` (keywords included).
    Ident(String),
    /// Integer literal: raw digits (underscores stripped) plus optional suffix.
    Int { value: String, suffix: Option<IntSuffix> },
    /// String literal contents (without the surrounding quotes), not normalized.
    Str(String),

    // --- punctuation ---
    Semi,
    Colon,
    ColonColon,
    Comma,
    Dot,
    Spread, // ...
    At,
    Arrow,    // ->
    FatArrow, // =>
    LBrace,
    RBrace,
    LParen,
    RParen,
    LBracket,
    RBracket,
    Lt,
    Gt,
    Le,
    Ge,
    EqEq,
    Ne,
    Eq,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Bang,
    AmpAmp,
    PipePipe,

    /// End of input.
    Eof,
}

/// A lexed token: its kind plus the source span it covers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

/// A lexing failure with a human-readable message and the offending span.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LexError {
    pub message: String,
    pub span: Span,
}

impl fmt::Display for LexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "lex error at {}..{}: {}",
            self.span.start, self.span.end, self.message
        )
    }
}

impl std::error::Error for LexError {}

/// Tokenize Edict source into a token stream terminated by [`TokenKind::Eof`].
///
/// # Errors
/// Returns a [`LexError`] on an unterminated string/comment or an unexpected
/// character.
pub fn lex(src: &str) -> Result<Vec<Token>, LexError> {
    Lexer::new(src).run()
}

struct Lexer<'a> {
    src: &'a [u8],
    pos: usize,
}

impl<'a> Lexer<'a> {
    fn new(src: &'a str) -> Self {
        Self { src: src.as_bytes(), pos: 0 }
    }

    fn run(mut self) -> Result<Vec<Token>, LexError> {
        let mut out = Vec::new();
        loop {
            self.skip_trivia()?;
            let start = self.pos;
            let Some(c) = self.peek() else {
                out.push(Token { kind: TokenKind::Eof, span: Span::new(start, start) });
                return Ok(out);
            };
            let kind = match c {
                b'a'..=b'z' | b'A'..=b'Z' | b'_' => self.ident(),
                b'0'..=b'9' => self.number(),
                b'"' => self.string()?,
                _ => self.punct()?,
            };
            out.push(Token { kind, span: Span::new(start, self.pos) });
        }
    }

    fn peek(&self) -> Option<u8> {
        self.src.get(self.pos).copied()
    }

    fn peek2(&self) -> Option<u8> {
        self.src.get(self.pos + 1).copied()
    }

    fn bump(&mut self) -> u8 {
        let c = self.src[self.pos];
        self.pos += 1;
        c
    }

    fn skip_trivia(&mut self) -> Result<(), LexError> {
        loop {
            match self.peek() {
                Some(b' ' | b'\t' | b'\r' | b'\n') => {
                    self.pos += 1;
                }
                Some(b'/') if self.peek2() == Some(b'/') => {
                    while let Some(c) = self.peek() {
                        if c == b'\n' {
                            break;
                        }
                        self.pos += 1;
                    }
                }
                Some(b'/') if self.peek2() == Some(b'*') => {
                    let start = self.pos;
                    self.pos += 2;
                    loop {
                        match self.peek() {
                            Some(b'*') if self.peek2() == Some(b'/') => {
                                self.pos += 2;
                                break;
                            }
                            Some(_) => self.pos += 1,
                            None => {
                                return Err(LexError {
                                    message: "unterminated block comment".into(),
                                    span: Span::new(start, self.pos),
                                });
                            }
                        }
                    }
                }
                _ => return Ok(()),
            }
        }
    }

    fn ident(&mut self) -> TokenKind {
        let start = self.pos;
        while matches!(self.peek(), Some(b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'_')) {
            self.pos += 1;
        }
        let text = std::str::from_utf8(&self.src[start..self.pos])
            .expect("ascii ident slice is valid utf8");
        TokenKind::Ident(text.to_string())
    }

    fn number(&mut self) -> TokenKind {
        let mut value = String::new();
        // digits with internal underscores
        while let Some(c) = self.peek() {
            match c {
                b'0'..=b'9' => {
                    value.push(char::from(c));
                    self.pos += 1;
                }
                b'_' => {
                    self.pos += 1;
                }
                _ => break,
            }
        }
        // optional type suffix immediately following the digits
        let suffix_start = self.pos;
        let mut suffix_text = String::new();
        while matches!(self.peek(), Some(b'a'..=b'z' | b'0'..=b'9')) {
            suffix_text.push(char::from(self.bump()));
        }
        let suffix = if suffix_text.is_empty() {
            None
        } else if let Some(s) = IntSuffix::from_str(&suffix_text) {
            Some(s)
        } else {
            // Not a valid suffix: rewind so the parser can flag it as a
            // separate token (keeps the lexer total).
            self.pos = suffix_start;
            None
        };
        TokenKind::Int { value, suffix }
    }

    fn string(&mut self) -> Result<TokenKind, LexError> {
        let open = self.pos;
        self.pos += 1; // opening quote
        let mut out = String::new();
        loop {
            match self.peek() {
                Some(b'"') => {
                    self.pos += 1;
                    return Ok(TokenKind::Str(out));
                }
                Some(b'\\') => {
                    self.pos += 1;
                    match self.peek() {
                        Some(b'"') => {
                            out.push('"');
                            self.pos += 1;
                        }
                        Some(b'\\') => {
                            out.push('\\');
                            self.pos += 1;
                        }
                        Some(b'n') => {
                            out.push('\n');
                            self.pos += 1;
                        }
                        Some(b't') => {
                            out.push('\t');
                            self.pos += 1;
                        }
                        _ => {
                            return Err(LexError {
                                message: "invalid string escape".into(),
                                span: Span::new(self.pos - 1, self.pos),
                            });
                        }
                    }
                }
                Some(_) => {
                    // copy one UTF-8 byte; multi-byte chars pass through intact
                    out.push(char::from(self.bump()));
                }
                None => {
                    return Err(LexError {
                        message: "unterminated string literal".into(),
                        span: Span::new(open, self.pos),
                    });
                }
            }
        }
    }

    fn punct(&mut self) -> Result<TokenKind, LexError> {
        let start = self.pos;
        let c = self.bump();
        let two = self.peek();
        let kind = match (c, two) {
            (b':', Some(b':')) => {
                self.pos += 1;
                TokenKind::ColonColon
            }
            (b'.', Some(b'.')) if self.peek2() == Some(b'.') => {
                self.pos += 2;
                TokenKind::Spread
            }
            (b'-', Some(b'>')) => {
                self.pos += 1;
                TokenKind::Arrow
            }
            (b'=', Some(b'>')) => {
                self.pos += 1;
                TokenKind::FatArrow
            }
            (b'=', Some(b'=')) => {
                self.pos += 1;
                TokenKind::EqEq
            }
            (b'!', Some(b'=')) => {
                self.pos += 1;
                TokenKind::Ne
            }
            (b'<', Some(b'=')) => {
                self.pos += 1;
                TokenKind::Le
            }
            (b'>', Some(b'=')) => {
                self.pos += 1;
                TokenKind::Ge
            }
            (b'&', Some(b'&')) => {
                self.pos += 1;
                TokenKind::AmpAmp
            }
            (b'|', Some(b'|')) => {
                self.pos += 1;
                TokenKind::PipePipe
            }
            (b';', _) => TokenKind::Semi,
            (b':', _) => TokenKind::Colon,
            (b',', _) => TokenKind::Comma,
            (b'.', _) => TokenKind::Dot,
            (b'@', _) => TokenKind::At,
            (b'{', _) => TokenKind::LBrace,
            (b'}', _) => TokenKind::RBrace,
            (b'(', _) => TokenKind::LParen,
            (b')', _) => TokenKind::RParen,
            (b'[', _) => TokenKind::LBracket,
            (b']', _) => TokenKind::RBracket,
            (b'<', _) => TokenKind::Lt,
            (b'>', _) => TokenKind::Gt,
            (b'=', _) => TokenKind::Eq,
            (b'+', _) => TokenKind::Plus,
            (b'-', _) => TokenKind::Minus,
            (b'*', _) => TokenKind::Star,
            (b'/', _) => TokenKind::Slash,
            (b'%', _) => TokenKind::Percent,
            (b'!', _) => TokenKind::Bang,
            _ => {
                return Err(LexError {
                    message: format!("unexpected character {:?}", char::from(c)),
                    span: Span::new(start, self.pos),
                });
            }
        };
        Ok(kind)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(src: &str) -> Vec<TokenKind> {
        lex(src).expect("lex ok").into_iter().map(|t| t.kind).collect()
    }

    #[test]
    fn lexes_package_ref() {
        let k = kinds("package examples.hello@1;");
        assert_eq!(
            k,
            vec![
                TokenKind::Ident("package".into()),
                TokenKind::Ident("examples".into()),
                TokenKind::Dot,
                TokenKind::Ident("hello".into()),
                TokenKind::At,
                TokenKind::Int { value: "1".into(), suffix: None },
                TokenKind::Semi,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn lexes_refined_scalar_and_ops() {
        let k = kinds("String<max=256> <= != + .");
        assert_eq!(
            k,
            vec![
                TokenKind::Ident("String".into()),
                TokenKind::Lt,
                TokenKind::Ident("max".into()),
                TokenKind::Eq,
                TokenKind::Int { value: "256".into(), suffix: None },
                TokenKind::Gt,
                TokenKind::Le,
                TokenKind::Ne,
                TokenKind::Plus,
                TokenKind::Dot,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn typed_integer_suffix() {
        let k = kinds("1u64 64_000i64 7");
        assert_eq!(
            k,
            vec![
                TokenKind::Int { value: "1".into(), suffix: Some(IntSuffix::U64) },
                TokenKind::Int { value: "64000".into(), suffix: Some(IntSuffix::I64) },
                TokenKind::Int { value: "7".into(), suffix: None },
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn skips_comments_and_keeps_strings() {
        let k = kinds("// line\n/* block */ \"hello, \"");
        assert_eq!(
            k,
            vec![TokenKind::Str("hello, ".into()), TokenKind::Eof]
        );
    }

    #[test]
    fn string_escapes_decode() {
        assert_eq!(
            kinds(r#""a\"b\\c""#),
            vec![TokenKind::Str("a\"b\\c".into()), TokenKind::Eof]
        );
    }

    #[test]
    fn unterminated_string_errors() {
        assert!(lex("\"oops").is_err());
    }
}
