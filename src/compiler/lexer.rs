use crate::compiler::{Location, Token, TokenKind};
use std::collections::VecDeque;
use std::io::{BufRead, Result};

trait IsOperatorExt {
    fn is_operator(&self) -> bool;
}

impl IsOperatorExt for char {
    fn is_operator(&self) -> bool {
        match *self {
            '~' | '&' | '|' | '*' | '/' | '\\' | '+' | '=' | '>' | '<' | ',' | '@' | '%' | '-' => {
                true
            }
            _ => false,
        }
    }
}

struct PeekableBuffer<R: BufRead> {
    reader: R,
    position: usize,
    line: usize,
    buffer: String,
}

impl<R: BufRead> PeekableBuffer<R> {
    fn new(mut reader: R) -> Result<PeekableBuffer<R>> {
        let mut buffer = String::new();
        reader.read_line(&mut buffer)?;
        Ok(PeekableBuffer {
            reader,
            buffer,
            position: 0,
            line: 1,
        })
    }

    fn peek(&mut self) -> Result<Option<char>> {
        let c = self.buffer.chars().nth(self.position);
        Ok(c)
    }

    fn consume(&mut self) -> Result<()> {
        self.position += 1;
        if self.position >= self.buffer.len() {
            self.buffer.clear();
            self.reader.read_line(&mut self.buffer)?;
            self.line += 1;
            self.position = 0;
        }

        Ok(())
    }

    fn current_location(&self) -> Location {
        Location {
            line: self.line,
            column: self.position,
        }
    }
}

pub struct Lexer<R: BufRead> {
    buffer: PeekableBuffer<R>,
    queue: VecDeque<Token>,
}

impl<R: BufRead> Iterator for Lexer<R> {
    type Item = Result<Token>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.read_token() {
            Ok(Some(t)) => Some(Ok(t)),
            Ok(None) => None,
            Err(e) => Some(Err(e)),
        }
    }
}

impl<R: BufRead> Lexer<R> {
    pub fn new(reader: R) -> Result<Lexer<R>> {
        Ok(Lexer {
            buffer: PeekableBuffer::new(reader)?,
            queue: VecDeque::new(),
        })
    }

    fn read_token(&mut self) -> Result<Option<Token>> {
        if !self.queue.is_empty() {
            return Ok(self.queue.pop_front());
        }

        loop {
            match self.buffer.peek()? {
                Some('\"') => self.skip_comment()?,
                Some(c) if c.is_whitespace() => self.buffer.consume()?,
                _ => break,
            }
        }

        let c = match self.buffer.peek()? {
            Some(c) => c,
            None => return Ok(None),
        };

        match c {
            '[' => self.read_symbol(TokenKind::NewBlock),
            ']' => self.read_symbol(TokenKind::EndBlock),
            '(' => self.read_symbol(TokenKind::NewTerm),
            ')' => self.read_symbol(TokenKind::EndTerm),
            '#' => self.read_symbol(TokenKind::Pound),
            '^' => self.read_symbol(TokenKind::Exit),
            '.' => self.read_symbol(TokenKind::Period),
            ':' => self.read_colon(),
            '\'' => self.read_string(),
            c if c.is_ascii_digit() => self.read_number(),
            c if c.is_ascii_alphabetic() => self.read_identifier(),
            c if c.is_operator() => self.read_operator(),
            c => panic!("do not understand: {}", c),
        }
    }

    fn read_colon(&mut self) -> Result<Option<Token>> {
        let location = self.buffer.current_location();
        self.buffer.consume()?;

        let kind = if let Some('=') = self.buffer.peek()? {
            self.buffer.consume()?;
            TokenKind::Assign
        } else {
            TokenKind::Colon
        };

        Ok(Some(Token::new(kind, None, location)))
    }

    fn read_identifier(&mut self) -> Result<Option<Token>> {
        let location = self.buffer.current_location();
        let mut text = String::new();

        loop {
            match self.buffer.peek()? {
                Some(c) if c.is_ascii_alphanumeric() || c == '_' => {
                    text.push(c);
                    self.buffer.consume()?;
                }
                _ => break,
            }
        }

        let token = if let Some(':') = self.buffer.peek()? {
            text.push(':');
            self.buffer.consume()?;

            match self.buffer.peek()? {
                Some(c) if c.is_ascii_alphabetic() => {
                    loop {
                        match self.buffer.peek()? {
                            Some(c) if c.is_ascii_alphabetic() || c == ':' => {
                                text.push(c);
                                self.buffer.consume()?;
                            }
                            _ => break,
                        }
                    }

                    Token::new(TokenKind::KeywordSequence, Some(text), location)
                }
                _ => Token::new(TokenKind::Keyword, Some(text), location),
            }
        } else if text == "primitive" {
            Token::new(TokenKind::Primitive, None, location)
        } else {
            Token::new(TokenKind::Identifier, Some(text), location)
        };

        Ok(Some(token))
    }

    fn read_number(&mut self) -> Result<Option<Token>> {
        let location = self.buffer.current_location();
        let mut text = String::new();

        loop {
            match self.buffer.peek()? {
                Some(c) if c.is_ascii_digit() => {
                    text.push(c);
                    self.buffer.consume()?;
                }
                _ => break,
            }
        }

        if let Some('.') = self.buffer.peek()? {
            let period_location = self.buffer.current_location();
            self.buffer.consume()?;

            match self.buffer.peek()? {
                Some(c) if c.is_ascii_digit() => {
                    text.push('.');

                    loop {
                        match self.buffer.peek()? {
                            Some(c) if c.is_ascii_digit() => {
                                text.push(c);
                                self.buffer.consume()?;
                            }
                            _ => break,
                        }
                    }

                    Ok(Some(Token::new(TokenKind::Double, Some(text), location)))
                }
                _ => {
                    self.queue
                        .push_back(Token::new(TokenKind::Period, None, period_location));
                    Ok(Some(Token::new(TokenKind::Integer, Some(text), location)))
                }
            }
        } else {
            Ok(Some(Token::new(TokenKind::Integer, Some(text), location)))
        }
    }

    fn read_operator(&mut self) -> Result<Option<Token>> {
        let location = self.buffer.current_location();
        let mut sequence = String::new();

        loop {
            match self.buffer.peek()? {
                Some(c) if c.is_operator() => {
                    self.buffer.consume()?;
                    sequence.push(c);
                }
                _ => break,
            }
        }

        if sequence.len() > 1 {
            if sequence.chars().all(|c| c == '-') && sequence.len() >= 4 {
                Ok(Some(Token::new(TokenKind::Separator, None, location)))
            } else {
                Ok(Some(Token::new(
                    TokenKind::OperatorSequence,
                    Some(sequence),
                    location,
                )))
            }
        } else {
            let c = sequence.chars().nth(0).unwrap();
            let kind = match c {
                '~' => TokenKind::Not,
                '&' => TokenKind::And,
                '|' => TokenKind::Or,
                '*' => TokenKind::Star,
                '/' => TokenKind::Divide,
                '\\' => TokenKind::Modulus,
                '+' => TokenKind::Plus,
                '=' => TokenKind::Equal,
                '>' => TokenKind::More,
                '<' => TokenKind::Less,
                ',' => TokenKind::Comma,
                '@' => TokenKind::At,
                '%' => TokenKind::Percent,
                '-' => TokenKind::Minus,
                _ => unreachable!(),
            };

            Ok(Some(Token::new(kind, Some(c.to_string()), location)))
        }
    }

    fn read_string(&mut self) -> Result<Option<Token>> {
        let location = self.buffer.current_location();
        let mut text = String::new();

        self.buffer.consume()?;

        loop {
            let c = self.buffer.peek()?;
            self.buffer.consume()?;
            match c {
                Some('\\') => {
                    let c = self.buffer.peek()?;
                    self.buffer.consume()?;
                    match c {
                        Some('\'') => text.push('\''),
                        Some('\\') => text.push('\\'),
                        Some('b') => text.push('\x08'),
                        Some('f') => text.push('\x0c'),
                        Some('n') => text.push('\n'),
                        Some('r') => text.push('\r'),
                        Some('t') => text.push('\t'),
                        _ => {}
                    }
                }
                Some(c) if c != '\'' => text.push(c),
                Some(_) => break,
                None => panic!("Parsing ended inside a string"),
            }
        }

        Ok(Some(Token::new(TokenKind::String, Some(text), location)))
    }

    fn read_symbol(&mut self, kind: TokenKind) -> Result<Option<Token>> {
        let location = self.buffer.current_location();
        self.buffer.consume()?;
        Ok(Some(Token::new(kind, None, location)))
    }

    fn skip_comment(&mut self) -> Result<()> {
        loop {
            self.buffer.consume()?;
            if let Some('"') = self.buffer.peek()? {
                self.buffer.consume()?;
                break;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skipping_whitespace() {
        let source = b"\n Hello \n Test";
        let mut lexer = Lexer::new(source.as_ref()).unwrap();

        let token = lexer.next().unwrap().unwrap();
        assert_eq!(TokenKind::Identifier, token.kind);
        assert_eq!("Hello", token.text.unwrap());

        let token = lexer.next().unwrap().unwrap();
        assert_eq!(TokenKind::Identifier, token.kind);
        assert_eq!("Test", token.text.unwrap());
    }

    #[test]
    fn skipping_comments() {
        let source = b"\"Test\" Hello \"123\"Test";
        let mut lexer = Lexer::new(source.as_ref()).unwrap();

        let token = lexer.next().unwrap().unwrap();
        assert_eq!(TokenKind::Identifier, token.kind);
        assert_eq!("Hello", token.text.unwrap());

        let token = lexer.next().unwrap().unwrap();
        assert_eq!(TokenKind::Identifier, token.kind);
        assert_eq!("Test", token.text.unwrap());
    }

    #[test]
    fn saves_current_location() {
        let source = b" \n  World";
        let mut lexer = Lexer::new(source.as_ref()).unwrap();
        let token = lexer.next().unwrap().unwrap();
        assert_eq!(Location { line: 2, column: 2 }, token.location);
    }

    #[test]
    fn reading_identifier() {
        let source = b"Test";
        let mut lexer = Lexer::new(source.as_ref()).unwrap();
        let token = lexer.next().unwrap().unwrap();
        assert_eq!(TokenKind::Identifier, token.kind);
        assert_eq!("Test", token.text.unwrap());
    }

    #[test]
    fn reading_keyword() {
        let source = b"test:";
        let mut lexer = Lexer::new(source.as_ref()).unwrap();
        let token = lexer.next().unwrap().unwrap();
        assert_eq!(TokenKind::Keyword, token.kind);
        assert_eq!("test:", token.text.unwrap());
    }

    #[test]
    fn reading_two_keyword_sequence() {
        let source = b"foo:bar:";
        let mut lexer = Lexer::new(source.as_ref()).unwrap();
        let token = lexer.next().unwrap().unwrap();
        assert_eq!(TokenKind::KeywordSequence, token.kind);
        assert_eq!("foo:bar:", token.text.unwrap());
    }

    #[test]
    fn reading_three_keyword_sequence() {
        let source = b"foo:bar:baz:";
        let mut lexer = Lexer::new(source.as_ref()).unwrap();
        let token = lexer.next().unwrap().unwrap();
        assert_eq!(TokenKind::KeywordSequence, token.kind);
        assert_eq!("foo:bar:baz:", token.text.unwrap());
    }

    #[test]
    fn reading_primitive() {
        let source = b"primitive";
        let mut lexer = Lexer::new(source.as_ref()).unwrap();
        let token = lexer.next().unwrap().unwrap();
        assert_eq!(TokenKind::Primitive, token.kind);
        assert_eq!(None, token.text);
    }

    #[test]
    fn reading_minus() {
        let source = b"-";
        let mut lexer = Lexer::new(source.as_ref()).unwrap();
        let token = lexer.next().unwrap().unwrap();
        assert_eq!(TokenKind::Minus, token.kind);
    }

    #[test]
    fn reading_two_minus() {
        let source = b"--";
        let mut lexer = Lexer::new(source.as_ref()).unwrap();
        let token = lexer.next().unwrap().unwrap();
        assert_eq!(TokenKind::OperatorSequence, token.kind);
        assert_eq!("--", token.text.unwrap());
    }

    #[test]
    fn reading_three_minus() {
        let source = b"---";
        let mut lexer = Lexer::new(source.as_ref()).unwrap();
        let token = lexer.next().unwrap().unwrap();
        assert_eq!(TokenKind::OperatorSequence, token.kind);
        assert_eq!("---", token.text.unwrap());
    }

    #[test]
    fn reading_minus_operator_sequence() {
        let source = b"-->";
        let mut lexer = Lexer::new(source.as_ref()).unwrap();
        let token = lexer.next().unwrap().unwrap();
        assert_eq!(TokenKind::OperatorSequence, token.kind);
        assert_eq!("-->", token.text.unwrap());
    }

    #[test]
    fn reading_separator() {
        let source = b"----";
        let mut lexer = Lexer::new(source.as_ref()).unwrap();
        let token = lexer.next().unwrap().unwrap();
        assert_eq!(TokenKind::Separator, token.kind);
    }

    #[test]
    fn reading_long_separator() {
        let source = b"----------------\ntest";
        let mut lexer = Lexer::new(source.as_ref()).unwrap();

        let token = lexer.next().unwrap().unwrap();
        assert_eq!(TokenKind::Separator, token.kind);

        let token = lexer.next().unwrap().unwrap();
        assert_eq!(TokenKind::Identifier, token.kind);
        assert_eq!("test", token.text.unwrap());
    }

    #[test]
    fn reading_integer() {
        let source = b"1";
        let mut lexer = Lexer::new(source.as_ref()).unwrap();
        let token = lexer.next().unwrap().unwrap();
        assert_eq!(TokenKind::Integer, token.kind);
        assert_eq!("1", token.text.unwrap());
    }

    #[test]
    fn reading_integer_and_period() {
        let source = b"1.";
        let mut lexer = Lexer::new(source.as_ref()).unwrap();

        let token = lexer.next().unwrap().unwrap();
        assert_eq!(TokenKind::Integer, token.kind);
        assert_eq!("1", token.text.unwrap());

        let token = lexer.next().unwrap().unwrap();
        assert_eq!(TokenKind::Period, token.kind);
    }

    #[test]
    fn reading_double() {
        let source = b"3.14";
        let mut lexer = Lexer::new(source.as_ref()).unwrap();
        let token = lexer.next().unwrap().unwrap();
        assert_eq!(TokenKind::Double, token.kind);
        assert_eq!("3.14", token.text.unwrap());
    }

    #[test]
    fn reading_string() {
        let source = b"'Hello'";
        let mut lexer = Lexer::new(source.as_ref()).unwrap();
        let token = lexer.next().unwrap().unwrap();
        assert_eq!(TokenKind::String, token.kind);
        assert_eq!("Hello", token.text.unwrap());
    }

    #[test]
    fn reading_unicode_string() {
        let source = "'ᚠᛇᚻ᛫ᛒᛦᚦ᛫ᚠᚱᚩᚠᚢᚱ᛫ᚠᛁᚱᚪ᛫ᚷᛖᚻᚹᛦᛚᚳᚢᛗ'".as_bytes();
        let mut lexer = Lexer::new(source).unwrap();
        let token = lexer.next().unwrap().unwrap();
        assert_eq!(TokenKind::String, token.kind);
        assert_eq!("ᚠᛇᚻ᛫ᛒᛦᚦ᛫ᚠᚱᚩᚠᚢᚱ᛫ᚠᛁᚱᚪ᛫ᚷᛖᚻᚹᛦᛚᚳᚢᛗ", token.text.unwrap());
    }

    #[test]
    fn reading_string_with_escape() {
        let source = b"'\\''";
        let mut lexer = Lexer::new(source.as_ref()).unwrap();
        let token = lexer.next().unwrap().unwrap();
        assert_eq!(TokenKind::String, token.kind);
        assert_eq!("'", token.text.unwrap());
    }

    #[test]
    fn reading_colon() {
        let source = b":";
        let mut lexer = Lexer::new(source.as_ref()).unwrap();
        let token = lexer.next().unwrap().unwrap();
        assert_eq!(TokenKind::Colon, token.kind);
    }

    #[test]
    fn reading_assignment() {
        let source = b"foo := 'Hello'";
        let mut lexer = Lexer::new(source.as_ref()).unwrap();

        let token = lexer.next().unwrap().unwrap();
        assert_eq!(TokenKind::Identifier, token.kind);
        assert_eq!("foo", token.text.unwrap());

        let token = lexer.next().unwrap().unwrap();
        assert_eq!(TokenKind::Assign, token.kind);

        let token = lexer.next().unwrap().unwrap();
        assert_eq!(TokenKind::String, token.kind);
        assert_eq!("Hello", token.text.unwrap());
    }

    #[test]
    fn reading_simple_symbols() {
        let source = b"[]()#^.";
        let mut lexer = Lexer::new(source.as_ref()).unwrap();

        assert_eq!(TokenKind::NewBlock, lexer.next().unwrap().unwrap().kind);
        assert_eq!(TokenKind::EndBlock, lexer.next().unwrap().unwrap().kind);
        assert_eq!(TokenKind::NewTerm, lexer.next().unwrap().unwrap().kind);
        assert_eq!(TokenKind::EndTerm, lexer.next().unwrap().unwrap().kind);
        assert_eq!(TokenKind::Pound, lexer.next().unwrap().unwrap().kind);
        assert_eq!(TokenKind::Exit, lexer.next().unwrap().unwrap().kind);
        assert_eq!(TokenKind::Period, lexer.next().unwrap().unwrap().kind);
    }

    #[test]
    fn reading_simple_operators() {
        let source = b"~ & | * / \\ + = < > , @ %";
        let mut lexer = Lexer::new(source.as_ref()).unwrap();

        assert_eq!(TokenKind::Not, lexer.next().unwrap().unwrap().kind);
        assert_eq!(TokenKind::And, lexer.next().unwrap().unwrap().kind);
        assert_eq!(TokenKind::Or, lexer.next().unwrap().unwrap().kind);
        assert_eq!(TokenKind::Star, lexer.next().unwrap().unwrap().kind);
        assert_eq!(TokenKind::Divide, lexer.next().unwrap().unwrap().kind);
        assert_eq!(TokenKind::Modulus, lexer.next().unwrap().unwrap().kind);
        assert_eq!(TokenKind::Plus, lexer.next().unwrap().unwrap().kind);
        assert_eq!(TokenKind::Equal, lexer.next().unwrap().unwrap().kind);
        assert_eq!(TokenKind::Less, lexer.next().unwrap().unwrap().kind);
        assert_eq!(TokenKind::More, lexer.next().unwrap().unwrap().kind);
        assert_eq!(TokenKind::Comma, lexer.next().unwrap().unwrap().kind);
        assert_eq!(TokenKind::At, lexer.next().unwrap().unwrap().kind);
        assert_eq!(TokenKind::Percent, lexer.next().unwrap().unwrap().kind);
    }

    #[test]
    fn reading_operator_sequence() {
        let source = b"<=";
        let mut lexer = Lexer::new(source.as_ref()).unwrap();
        let token = lexer.next().unwrap().unwrap();
        assert_eq!(TokenKind::OperatorSequence, token.kind);
        assert_eq!("<=", token.text.unwrap());
    }

    #[test]
    fn reading_example_program() {
        let source = b"
        Hello = (
            \"The 'run' method is called when initializing the system\"
            run = ('Hello, World from SOM' println)
        )
        ";
        let mut lexer = Lexer::new(source.as_ref()).unwrap();

        let token = lexer.next().unwrap().unwrap();
        assert_eq!(TokenKind::Identifier, token.kind);
        assert_eq!("Hello", token.text.unwrap());

        let token = lexer.next().unwrap().unwrap();
        assert_eq!(TokenKind::Equal, token.kind);

        let token = lexer.next().unwrap().unwrap();
        assert_eq!(TokenKind::NewTerm, token.kind);

        let token = lexer.next().unwrap().unwrap();
        assert_eq!(TokenKind::Identifier, token.kind);
        assert_eq!("run", token.text.unwrap());

        let token = lexer.next().unwrap().unwrap();
        assert_eq!(TokenKind::Equal, token.kind);

        let token = lexer.next().unwrap().unwrap();
        assert_eq!(TokenKind::NewTerm, token.kind);

        let token = lexer.next().unwrap().unwrap();
        assert_eq!(TokenKind::String, token.kind);
        assert_eq!("Hello, World from SOM", token.text.unwrap());

        let token = lexer.next().unwrap().unwrap();
        assert_eq!(TokenKind::Identifier, token.kind);
        assert_eq!("println", token.text.unwrap());

        let token = lexer.next().unwrap().unwrap();
        assert_eq!(TokenKind::EndTerm, token.kind);

        let token = lexer.next().unwrap().unwrap();
        assert_eq!(TokenKind::EndTerm, token.kind);

        assert!(lexer.next().is_none());
    }
}
