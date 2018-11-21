use crate::compiler::{ast, Lexer, Location, Token, TokenKind};
use std::collections::HashMap;
use std::io::{self, BufRead};
use std::iter::Peekable;
use std::path::Path;
use std::result;

#[derive(Debug)]
pub enum Error {
    ParseError {
        description: String,
        filename: String,
        location: Location,
    },
    IoError(io::Error),
}

impl From<io::Error> for Error {
    fn from(source: io::Error) -> Self {
        Error::IoError(source)
    }
}

pub type Result<T> = result::Result<T, Error>;

const SYMBOL_KINDS: [TokenKind; 19] = [
    TokenKind::Identifier,
    TokenKind::String,
    TokenKind::Keyword,
    TokenKind::KeywordSequence,
    TokenKind::OperatorSequence,
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

pub struct Parser<R: BufRead> {
    lexer: Peekable<Lexer<R>>,
    filename: String,
    last_location: Location,
}

impl<R: BufRead> Parser<R> {
    pub fn new<P: AsRef<Path>>(reader: R, filename: P) -> Result<Parser<R>> {
        Ok(Parser {
            lexer: Lexer::new(reader)?.peekable(),
            filename: filename.as_ref().to_string_lossy().into_owned(),
            last_location: Location::default(),
        })
    }

    pub fn parse(&mut self) -> Result<ast::Class> {
        let name = self.expect_token(TokenKind::Identifier)?.text.unwrap();
        let _ = self.expect_token(TokenKind::Equal)?;

        let superclass = if self.peek_token_kind()? == TokenKind::Identifier {
            self.expect_token(TokenKind::Identifier)?.text
        } else {
            None
        };

        let _ = self.expect_token(TokenKind::NewTerm)?;

        let instance_variables = self.parse_locals()?;
        let instance_methods = self.parse_methods()?;

        let class_variables;
        let class_methods;
        if let Ok(TokenKind::Separator) = self.peek_token_kind() {
            let _ = self.expect_token(TokenKind::Separator)?;
            class_variables = self.parse_locals()?;
            class_methods = self.parse_methods()?;
        } else {
            class_variables = vec![];
            class_methods = HashMap::new();
        }

        Ok(ast::Class {
            name,
            superclass,
            class_methods,
            class_variables,
            instance_methods,
            instance_variables,
        })
    }

    fn parse_block_parameters(&mut self) -> Result<Vec<String>> {
        let mut parameters = vec![];

        while let TokenKind::Colon = self.peek_token_kind()? {
            let _ = self.expect_token(TokenKind::Colon)?;
            let parameter = self.expect_token(TokenKind::Identifier)?.text.unwrap();
            parameters.push(parameter);
        }

        if !parameters.is_empty() {
            let _ = self.expect_token(TokenKind::Or)?;
        }

        Ok(parameters)
    }

    fn parse_body(&mut self) -> Result<Vec<ast::Expression>> {
        let mut expressions = vec![];
        loop {
            match self.peek_token_kind()? {
                TokenKind::EndTerm => break,
                TokenKind::EndBlock => break,
                TokenKind::Exit => expressions.push(self.parse_expression_result()?),
                _ => expressions.push(self.parse_expression()?),
            };

            if let TokenKind::Period = self.peek_token_kind()? {
                let _ = self.expect_token(TokenKind::Period)?;
            }
        }

        Ok(expressions)
    }

    fn parse_expression(&mut self) -> Result<ast::Expression> {
        let mut expression = self.parse_expression_primary()?;
        loop {
            expression = match self.peek_token_kind()? {
                TokenKind::Assign => self.parse_expression_assignment(expression)?,
                TokenKind::Identifier => self.parse_expression_messages(expression)?,
                TokenKind::Keyword => self.parse_expression_messages(expression)?,
                TokenKind::OperatorSequence => self.parse_expression_messages(expression)?,
                kind if kind.is_binary_operator() => self.parse_expression_messages(expression)?,
                _ => break,
            }
        }

        Ok(expression)
    }

    fn parse_expression_array(&mut self) -> Result<ast::Expression> {
        let mut values = vec![];

        let _ = self.expect_token(TokenKind::NewTerm)?;
        loop {
            match self.peek_token_kind()? {
                TokenKind::EndTerm => break,
                _ => values.push(self.parse_expression()?),
            }
        }

        let _ = self.expect_token(TokenKind::EndTerm)?;

        Ok(ast::Expression::LiteralArray(values))
    }

    fn parse_expression_assignment(&mut self, left: ast::Expression) -> Result<ast::Expression> {
        let token = self.expect_token(TokenKind::Assign)?;

        if let ast::Expression::Variable(name) = left {
            let right = self.parse_expression()?;
            let expression = ast::Expression::Assignment {
                variable: name,
                value: Box::new(right),
            };

            Ok(expression)
        } else {
            Err(Error::ParseError {
                description:
                    "Attempt to assign result of expression to something other than a variable"
                        .into(),
                filename: self.filename.clone(),
                location: token.location,
            })
        }
    }

    fn parse_expression_binary_message(
        &mut self,
        left: ast::Expression,
    ) -> Result<ast::Expression> {
        let kind = self.peek_token_kind()?;
        let message = self.expect_token(kind)?.text.unwrap();
        let right = self.parse_expression_binary_operand()?;
        let expression = ast::Expression::BinaryMessage {
            message,
            left: Box::new(left),
            right: Box::new(right),
        };

        Ok(expression)
    }

    fn parse_expression_binary_operand(&mut self) -> Result<ast::Expression> {
        let mut value = self.parse_expression_primary()?;

        while let TokenKind::Identifier = self.peek_token_kind()? {
            value = self.parse_expression_unary_message(value)?;
        }

        Ok(value)
    }

    fn parse_expression_formula(&mut self) -> Result<ast::Expression> {
        let mut value = self.parse_expression_binary_operand()?;

        loop {
            match self.peek_token_kind()? {
                TokenKind::OperatorSequence => {
                    value = self.parse_expression_binary_message(value)?
                }
                kind if kind.is_binary_operator() => {
                    value = self.parse_expression_binary_message(value)?
                }
                _ => break,
            }
        }

        Ok(value)
    }

    fn parse_expression_identifier(&mut self) -> Result<ast::Expression> {
        let name = self.expect_token(TokenKind::Identifier)?.text.unwrap();
        let expression = match name.as_str() {
            "false" => ast::Expression::LiteralBoolean(false),
            "nil" => ast::Expression::LiteralNil,
            "true" => ast::Expression::LiteralBoolean(true),
            _ => ast::Expression::Variable(name),
        };

        Ok(expression)
    }

    fn parse_expression_keyword_message(
        &mut self,
        value: ast::Expression,
    ) -> Result<ast::Expression> {
        let mut message = String::new();
        let mut parameters = Vec::new();

        while let TokenKind::Keyword = self.peek_token_kind()? {
            let keyword = self.expect_token(TokenKind::Keyword)?.text.unwrap();
            let parameter = self.parse_expression_formula()?;

            message.push_str(&keyword);
            parameters.push(parameter);
        }

        Ok(ast::Expression::KeywordMessage {
            receiver: Box::new(value),
            message,
            parameters,
        })
    }

    fn parse_expression_messages(&mut self, value: ast::Expression) -> Result<ast::Expression> {
        match self.peek_token_kind()? {
            TokenKind::Identifier => self.parse_expression_unary_message(value),
            TokenKind::Keyword => self.parse_expression_keyword_message(value),
            TokenKind::OperatorSequence => self.parse_expression_binary_message(value),
            kind if kind.is_binary_operator() => self.parse_expression_binary_message(value),
            _ => unreachable!(),
        }
    }

    fn parse_expression_negative_number(&mut self) -> Result<ast::Expression> {
        let _ = self.expect_token(TokenKind::Minus)?;
        self.parse_expression_number(true)
    }

    fn parse_expression_nested_block(&mut self) -> Result<ast::Expression> {
        let _ = self.expect_token(TokenKind::NewBlock)?;
        let expression = ast::Expression::Block {
            parameters: self.parse_block_parameters()?,
            locals: self.parse_locals()?,
            body: self.parse_body()?,
        };

        let _ = self.expect_token(TokenKind::EndBlock)?;

        Ok(expression)
    }

    fn parse_expression_nested_term(&mut self) -> Result<ast::Expression> {
        let _ = self.expect_token(TokenKind::NewTerm)?;
        let expression = self.parse_expression()?;
        let _ = self.expect_token(TokenKind::EndTerm)?;

        Ok(expression)
    }

    fn parse_expression_number(&mut self, negative: bool) -> Result<ast::Expression> {
        let token = self.expect_token_one_of(&[TokenKind::Integer, TokenKind::Double])?;
        match token {
            Token {
                kind: TokenKind::Integer,
                text: Some(text),
                ..
            } => {
                let mut value: i64 = text.parse().unwrap();
                if negative {
                    value = -value;
                }

                Ok(ast::Expression::LiteralInteger(value))
            }
            Token {
                kind: TokenKind::Double,
                text: Some(text),
                ..
            } => {
                let mut value: f64 = text.parse().unwrap();
                if negative {
                    value = -value;
                }

                Ok(ast::Expression::LiteralDouble(value))
            }
            _ => unreachable!(),
        }
    }

    fn parse_expression_pound(&mut self) -> Result<ast::Expression> {
        let _ = self.expect_token(TokenKind::Pound)?;
        if let TokenKind::NewTerm = self.peek_token_kind()? {
            self.parse_expression_array()
        } else {
            self.parse_expression_symbol()
        }
    }

    fn parse_expression_primary(&mut self) -> Result<ast::Expression> {
        match self.peek_token_kind()? {
            TokenKind::Double => self.parse_expression_number(false),
            TokenKind::Identifier => self.parse_expression_identifier(),
            TokenKind::Integer => self.parse_expression_number(false),
            TokenKind::Minus => self.parse_expression_negative_number(),
            TokenKind::NewBlock => self.parse_expression_nested_block(),
            TokenKind::NewTerm => self.parse_expression_nested_term(),
            TokenKind::Pound => self.parse_expression_pound(),
            TokenKind::String => self.parse_expression_string(),
            k => unreachable!("Unknown expression token: {:?}", k),
        }
    }

    fn parse_expression_result(&mut self) -> Result<ast::Expression> {
        let _ = self.expect_token(TokenKind::Exit)?;
        let statement = Box::new(self.parse_expression()?);
        Ok(ast::Expression::Return(statement))
    }

    fn parse_expression_string(&mut self) -> Result<ast::Expression> {
        let value = self.expect_token(TokenKind::String)?.text.unwrap();
        let expression = ast::Expression::LiteralString(value);

        Ok(expression)
    }

    fn parse_expression_symbol(&mut self) -> Result<ast::Expression> {
        let value = self.expect_token_one_of(&SYMBOL_KINDS)?.text.unwrap();
        let expression = ast::Expression::LiteralSymbol(value);

        Ok(expression)
    }

    fn parse_expression_unary_message(
        &mut self,
        value: ast::Expression,
    ) -> Result<ast::Expression> {
        let name = self.expect_token(TokenKind::Identifier)?.text.unwrap();
        let expression = ast::Expression::UnaryMessage {
            receiver: Box::new(value),
            message: name,
        };

        Ok(expression)
    }

    fn parse_locals(&mut self) -> Result<Vec<String>> {
        let mut locals = vec![];
        if let Ok(TokenKind::Or) = self.peek_token_kind() {
            self.expect_token(TokenKind::Or)?;

            while let Ok(TokenKind::Identifier) = self.peek_token_kind() {
                let name = self.expect_token(TokenKind::Identifier)?.text.unwrap();
                locals.push(name);
            }

            self.expect_token(TokenKind::Or)?;
        }

        Ok(locals)
    }

    fn parse_methods(&mut self) -> Result<HashMap<String, ast::Method>> {
        let mut methods = HashMap::new();

        loop {
            let method = match self.peek_token_kind()? {
                TokenKind::Identifier => try!(self.parse_method()),
                TokenKind::Keyword => try!(self.parse_method()),
                TokenKind::OperatorSequence => try!(self.parse_method()),
                kind if kind.is_binary_operator() => try!(self.parse_method()),
                _ => break,
            };

            let name = match &method {
                ast::Method::Primitive { name, .. } => name.clone(),
                ast::Method::Native { name, .. } => name.clone(),
            };

            methods.insert(name, method);
        }

        Ok(methods)
    }

    fn parse_method(&mut self) -> Result<ast::Method> {
        let (name, parameters) = self.parse_pattern()?;
        let _ = self.expect_token(TokenKind::Equal)?;

        let method = if let TokenKind::Primitive = self.peek_token_kind()? {
            let _ = self.expect_token(TokenKind::Primitive)?;
            ast::Method::Primitive { name, parameters }
        } else {
            let _ = self.expect_token(TokenKind::NewTerm)?;
            let method = ast::Method::Native {
                name,
                parameters,
                locals: try!(self.parse_locals()),
                body: try!(self.parse_body()),
            };

            let _ = self.expect_token(TokenKind::EndTerm)?;

            method
        };

        Ok(method)
    }

    fn parse_pattern(&mut self) -> Result<(String, Vec<String>)> {
        match self.peek_token_kind()? {
            TokenKind::Identifier => self.parse_unary_pattern(),
            TokenKind::Keyword => self.parse_keyword_pattern(),
            TokenKind::OperatorSequence => self.parse_binary_pattern(),
            kind if kind.is_binary_operator() => self.parse_binary_pattern(),
            _ => unreachable!(),
        }
    }

    fn parse_unary_pattern(&mut self) -> Result<(String, Vec<String>)> {
        let name = self.expect_token(TokenKind::Identifier)?.text.unwrap();
        Ok((name, vec![]))
    }

    fn parse_keyword_pattern(&mut self) -> Result<(String, Vec<String>)> {
        let mut name = self.expect_token(TokenKind::Keyword)?.text.unwrap();
        let mut parameters = vec![];
        parameters.push(self.expect_token(TokenKind::Identifier)?.text.unwrap());

        while let TokenKind::Keyword = self.peek_token_kind()? {
            let part = self.expect_token(TokenKind::Keyword)?.text.unwrap();
            let parameter = self.expect_token(TokenKind::Identifier)?.text.unwrap();

            name.push_str(&part);
            parameters.push(parameter);
        }

        Ok((name, parameters))
    }

    fn parse_binary_pattern(&mut self) -> Result<(String, Vec<String>)> {
        let kind = self.peek_token_kind()?;
        let message = self.expect_token(kind)?.text.unwrap();
        let parameter = self.expect_token(TokenKind::Identifier)?.text.unwrap();

        Ok((message, vec![parameter]))
    }

    fn peek_token_kind(&mut self) -> Result<TokenKind> {
        match self.lexer.peek() {
            Some(Ok(t)) => Ok(t.kind),
            _ => Err(Error::ParseError {
                description: "Unexpected end of program".into(),
                filename: self.filename.clone(),
                location: self.last_location,
            }),
        }
    }

    fn expect_token(&mut self, kind: TokenKind) -> Result<Token> {
        self.expect_token_one_of(&[kind])
    }

    fn expect_token_one_of(&mut self, expected: &[TokenKind]) -> Result<Token> {
        let token = self.lexer.next();
        match token {
            Some(Ok(t)) => {
                self.last_location = t.location;
                if expected.contains(&t.kind) {
                    Ok(t)
                } else {
                    Err(Error::ParseError {
                        description: format!("Expected {:?}, found {:?}", expected, t.kind),
                        filename: self.filename.clone(),
                        location: t.location,
                    })
                }
            }
            Some(Err(e)) => Err(e.into()),
            None => Err(Error::ParseError {
                description: "Unexpected end of program".into(),
                filename: self.filename.clone(),
                location: self.last_location,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_with_simple_class() {
        let source = b"Hello = ()";
        let mut parser = Parser::new(source.as_ref(), "test").unwrap();

        let class = parser.parse().unwrap();
        assert_eq!("Hello", class.name);
        assert_eq!(None, class.superclass);
    }

    #[test]
    fn test_parse_with_superclass() {
        let source = b"Hello = Test ()";
        let mut parser = Parser::new(source.as_ref(), "test").unwrap();

        let class = parser.parse().unwrap();
        assert_eq!("Hello", class.name);
        assert_eq!("Test", class.superclass.unwrap());
    }

    #[test]
    fn test_parse_class_with_variables() {
        let source = b"
        Hello = (
            | foo bar |
            ----
            | baz qux |
        )";
        let mut parser = Parser::new(source.as_ref(), "test").unwrap();

        let class = parser.parse().unwrap();
        assert_eq!(vec!["foo", "bar"], class.instance_variables);
        assert_eq!(vec!["baz", "qux"], class.class_variables);
    }

    #[test]
    fn test_parse_class_with_primitive_methods() {
        let source = b"
        Hello = (
            foo = primitive
            ----
            bar: a baz: b = primitive
        )";
        let mut parser = Parser::new(source.as_ref(), "test").unwrap();
        let class = parser.parse().unwrap();

        let method = class.instance_methods.get("foo").unwrap();
        assert_eq!(
            &ast::Method::Primitive {
                name: "foo".into(),
                parameters: vec![],
            },
            method
        );

        let method = class.class_methods.get("bar:baz:").unwrap();
        assert_eq!(
            &ast::Method::Primitive {
                name: "bar:baz:".into(),
                parameters: vec!["a".into(), "b".into()],
            },
            method
        );
    }

    #[test]
    fn test_parse_expression_integer_literal() {
        let source = b"1.";
        let mut parser = Parser::new(source.as_ref(), "test").unwrap();
        let expression = parser.parse_expression().unwrap();
        assert_eq!(ast::Expression::LiteralInteger(1), expression);
    }

    #[test]
    fn test_parse_expression_negative_integer_literal() {
        let source = b"-1.";
        let mut parser = Parser::new(source.as_ref(), "test").unwrap();
        let expression = parser.parse_expression().unwrap();
        assert_eq!(ast::Expression::LiteralInteger(-1), expression);
    }

    #[test]
    fn test_parse_expression_double_literal() {
        let source = b"1.23.";
        let mut parser = Parser::new(source.as_ref(), "test").unwrap();
        let expression = parser.parse_expression().unwrap();
        assert_eq!(ast::Expression::LiteralDouble(1.23), expression);
    }

    #[test]
    fn test_parse_expression_negative_double_literal() {
        let source = b"-1.23.";
        let mut parser = Parser::new(source.as_ref(), "test").unwrap();
        let expression = parser.parse_expression().unwrap();
        assert_eq!(ast::Expression::LiteralDouble(-1.23), expression);
    }

    #[test]
    fn test_parse_expression_variable() {
        let source = b"a.";
        let mut parser = Parser::new(source.as_ref(), "test").unwrap();
        let expression = parser.parse_expression().unwrap();
        assert_eq!(ast::Expression::Variable("a".into()), expression);
    }

    #[test]
    fn test_parse_expression_string_literal() {
        let source = b"'test'.";
        let mut parser = Parser::new(source.as_ref(), "test").unwrap();
        let expression = parser.parse_expression().unwrap();
        assert_eq!(expression, ast::Expression::LiteralString("test".into()));
    }

    #[test]
    fn test_parse_expression_nil_literal() {
        let source = b"nil.";
        let mut parser = Parser::new(source.as_ref(), "test").unwrap();
        let expression = parser.parse_expression().unwrap();
        assert_eq!(ast::Expression::LiteralNil, expression);
    }

    #[test]
    fn test_parse_expression_array_literal() {
        let source = b"#(1 2).";
        let mut parser = Parser::new(source.as_ref(), "test").unwrap();
        let expression = parser.parse_expression().unwrap();
        assert_eq!(
            ast::Expression::LiteralArray(vec![
                ast::Expression::LiteralInteger(1),
                ast::Expression::LiteralInteger(2)
            ]),
            expression
        );
    }

    #[test]
    fn test_parse_expression_unary_message() {
        let source = b"1 println.";
        let mut parser = Parser::new(source.as_ref(), "test").unwrap();
        let expression = parser.parse_expression().unwrap();
        assert_eq!(
            ast::Expression::UnaryMessage {
                message: "println".into(),
                receiver: Box::new(ast::Expression::LiteralInteger(1)),
            },
            expression
        );
    }

    #[test]
    fn test_parse_expression_multiple_unary_messages() {
        let source = b"1 test println.";
        let mut parser = Parser::new(source.as_ref(), "test").unwrap();
        let expression = parser.parse_expression().unwrap();
        assert_eq!(
            ast::Expression::UnaryMessage {
                message: "println".into(),
                receiver: Box::new(ast::Expression::UnaryMessage {
                    message: "test".into(),
                    receiver: Box::new(ast::Expression::LiteralInteger(1)),
                }),
            },
            expression
        );
    }

    #[test]
    fn test_parse_expression_binary_operator() {
        let source = b"1 + 2.";
        let mut parser = Parser::new(source.as_ref(), "test").unwrap();
        let expression = parser.parse_expression().unwrap();
        assert_eq!(
            ast::Expression::BinaryMessage {
                message: "+".into(),
                left: Box::new(ast::Expression::LiteralInteger(1)),
                right: Box::new(ast::Expression::LiteralInteger(2)),
            },
            expression
        );
    }

    #[test]
    fn test_parse_expression_operator_sequence() {
        let source = b"1 <= 2.";
        let mut parser = Parser::new(source.as_ref(), "test").unwrap();
        let expression = parser.parse_expression().unwrap();
        assert_eq!(
            ast::Expression::BinaryMessage {
                message: "<=".into(),
                left: Box::new(ast::Expression::LiteralInteger(1)),
                right: Box::new(ast::Expression::LiteralInteger(2)),
            },
            expression
        );
    }

    #[test]
    fn test_parse_expression_boolean_literals() {
        let source = b"true || false.";
        let mut parser = Parser::new(source.as_ref(), "test").unwrap();
        let expression = parser.parse_expression().unwrap();
        assert_eq!(
            ast::Expression::BinaryMessage {
                message: "||".into(),
                left: Box::new(ast::Expression::LiteralBoolean(true)),
                right: Box::new(ast::Expression::LiteralBoolean(false)),
            },
            expression
        );
    }

    #[test]
    fn test_parse_expression_complex_messages() {
        let source = b"1 with: a length and: 1 + 2.";
        let mut parser = Parser::new(source.as_ref(), "test").unwrap();
        let expression = parser.parse_expression().unwrap();
        assert_eq!(
            ast::Expression::KeywordMessage {
                message: "with:and:".into(),
                parameters: vec![
                    ast::Expression::UnaryMessage {
                        message: "length".into(),
                        receiver: Box::new(ast::Expression::Variable("a".into())),
                    },
                    ast::Expression::BinaryMessage {
                        message: "+".into(),
                        left: Box::new(ast::Expression::LiteralInteger(1)),
                        right: Box::new(ast::Expression::LiteralInteger(2)),
                    },
                ],
                receiver: Box::new(ast::Expression::LiteralInteger(1)),
            },
            expression
        );
    }

    #[test]
    fn test_parse_expression_assignment() {
        let source = b"a := 'test'.";
        let mut parser = Parser::new(source.as_ref(), "test").unwrap();
        let expression = parser.parse_expression().unwrap();
        assert_eq!(
            ast::Expression::Assignment {
                variable: "a".into(),
                value: Box::new(ast::Expression::LiteralString("test".into())),
            },
            expression
        );
    }

    #[test]
    fn test_parse_assignment_error() {
        let source = b"1 := 'test'.";
        let mut parser = Parser::new(source.as_ref(), "test").unwrap();
        let result = parser.parse_expression().unwrap_err();

        if let Error::ParseError {
            description,
            filename,
            location,
        } = result
        {
            assert_eq!(
                "Attempt to assign result of expression to something other than a variable",
                description
            );
            assert_eq!("test", filename);
            assert_eq!(Location { column: 2, line: 1 }, location);
        } else {
            panic!("Other failure");
        }
    }

    #[test]
    fn test_parse_multiple_assignment() {
        let source = b"a := b := 'test'.";
        let mut parser = Parser::new(source.as_ref(), "test").unwrap();
        let expression = parser.parse_expression().unwrap();
        assert_eq!(
            ast::Expression::Assignment {
                variable: "a".into(),
                value: Box::new(ast::Expression::Assignment {
                    variable: "b".into(),
                    value: Box::new(ast::Expression::LiteralString("test".into())),
                }),
            },
            expression
        );
    }

    #[test]
    fn test_parse_expression_nested_terms() {
        let source = b"1 + (2 - 1).";
        let mut parser = Parser::new(source.as_ref(), "test").unwrap();
        let expression = parser.parse_expression().unwrap();
        assert_eq!(
            ast::Expression::BinaryMessage {
                message: "+".into(),
                left: Box::new(ast::Expression::LiteralInteger(1)),
                right: Box::new(ast::Expression::BinaryMessage {
                    message: "-".into(),
                    left: Box::new(ast::Expression::LiteralInteger(2)),
                    right: Box::new(ast::Expression::LiteralInteger(1)),
                }),
            },
            expression
        );
    }

    #[test]
    fn test_parse_expression_unary_message_binds_highest() {
        let source = b"1 test + 2.";
        let mut parser = Parser::new(source.as_ref(), "test").unwrap();
        let expression = parser.parse_expression().unwrap();
        assert_eq!(
            ast::Expression::BinaryMessage {
                message: "+".into(),
                left: Box::new(ast::Expression::UnaryMessage {
                    receiver: Box::new(ast::Expression::LiteralInteger(1)),
                    message: "test".into(),
                }),
                right: Box::new(ast::Expression::LiteralInteger(2)),
            },
            expression
        );
    }

    #[test]
    fn test_parse_expression_literal_symbols() {
        let source = b"#test #'test-case' #run:with:.";
        let mut parser = Parser::new(source.as_ref(), "test").unwrap();

        let expression = parser.parse_expression().unwrap();
        assert_eq!(ast::Expression::LiteralSymbol("test".into()), expression);

        let expression = parser.parse_expression().unwrap();
        assert_eq!(
            ast::Expression::LiteralSymbol("test-case".into()),
            expression
        );

        let expression = parser.parse_expression().unwrap();
        assert_eq!(
            ast::Expression::LiteralSymbol("run:with:".into()),
            expression
        );
    }

    #[test]
    fn test_parse_method_with_exit() {
        let source = b"
        test = (
            ^ 1 + 1.
        )";
        let mut parser = Parser::new(source.as_ref(), "test").unwrap();
        let method = parser.parse_method().unwrap();
        assert_eq!(
            ast::Method::Native {
                name: "test".into(),
                parameters: vec![],
                locals: vec![],
                body: vec![ast::Expression::Return(Box::new(
                    ast::Expression::BinaryMessage {
                        message: "+".into(),
                        left: Box::new(ast::Expression::LiteralInteger(1)),
                        right: Box::new(ast::Expression::LiteralInteger(1)),
                    },
                )),],
            },
            method
        );
    }

    #[test]
    fn test_parse_echo_program() {
        let source = b"
        Echo = (
            run: args = (
                args from: 2 to: args length do: [ :arg | arg print. ' ' print ].
                '' println.
            )
        )";
        let mut parser = Parser::new(source.as_ref(), "test").unwrap();

        let class = parser.parse().unwrap();
        assert_eq!("Echo", class.name);
        assert_eq!(None, class.superclass);

        if let Some(ast::Method::Native { name, locals, .. }) = class.instance_methods.get("run:") {
            assert_eq!("run:", name);
            assert!(locals.is_empty());
        } else {
            panic!("No method")
        }
    }
}
