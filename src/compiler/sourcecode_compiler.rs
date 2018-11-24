use crate::compiler::{ast, ParseError, Parser};
use crate::vmobjects::SClass;
use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufReader};
use std::path::Path;

#[derive(Debug)]
pub enum CompileError {
    ParseError(ParseError),
    IoError(io::Error),
}

impl From<ParseError> for CompileError {
    fn from(source: ParseError) -> Self {
        CompileError::ParseError(source)
    }
}

impl From<io::Error> for CompileError {
    fn from(source: io::Error) -> Self {
        CompileError::IoError(source)
    }
}

pub fn compile_path<P: AsRef<Path>>(path: P) -> Result<SClass, CompileError> {
    let file = File::open(&path)?;
    let reader = BufReader::new(file);
    let mut parser = Parser::new(reader, path);
    let class = parser.parse()?;

    compile(class)
}

fn compile(class: ast::Class) -> Result<SClass, CompileError> {
    Ok(SClass {
        name: class.name,
        superclass: None,
        invokables: HashMap::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compile_simple_class() {}
}
