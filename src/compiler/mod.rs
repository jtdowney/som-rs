pub mod ast;
mod lexer;
mod parser;
pub mod sourcecode_compiler;
mod token;

pub use self::lexer::Lexer;
pub use self::parser::{ParseError, Parser};
pub use self::sourcecode_compiler::compile_path;
pub use self::token::{Token, TokenKind};

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Location {
    pub line: usize,
    pub column: usize,
}

impl Default for Location {
    fn default() -> Self {
        Location { line: 1, column: 0 }
    }
}
