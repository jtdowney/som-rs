use crate::compiler::Location;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum TokenKind {
    And,
    Assign,
    At,
    Colon,
    Comma,
    Divide,
    Double,
    EndBlock,
    EndTerm,
    Equal,
    Exit,
    Identifier,
    Integer,
    Keyword,
    KeywordSequence,
    Less,
    Minus,
    Modulus,
    More,
    NewBlock,
    NewTerm,
    Not,
    OperatorSequence,
    Or,
    Percent,
    Period,
    Plus,
    Pound,
    Primitive,
    Separator,
    Star,
    String,
}

const BINARY_OPERATORS: [TokenKind; 14] = [
    TokenKind::And,
    TokenKind::At,
    TokenKind::Comma,
    TokenKind::Divide,
    TokenKind::Equal,
    TokenKind::Less,
    TokenKind::Minus,
    TokenKind::Modulus,
    TokenKind::More,
    TokenKind::Not,
    TokenKind::Or,
    TokenKind::Percent,
    TokenKind::Plus,
    TokenKind::Star,
];

impl TokenKind {
    pub fn is_binary_operator(self) -> bool {
        BINARY_OPERATORS.contains(&self)
    }
}

#[derive(Debug, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub text: Option<String>,
    pub location: Location,
}

impl Token {
    pub fn new(kind: TokenKind, text: Option<String>, location: Location) -> Token {
        Token {
            kind,
            text,
            location,
        }
    }
}
