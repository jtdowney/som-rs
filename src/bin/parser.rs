extern crate som;

use som::compiler::{ast, Parser};
use std::env;
use std::fs::File;
use std::io::BufReader;

#[cfg_attr(tarpaulin, skip)]
fn main() {
    let filename = env::args().nth(1).expect("filename to lex");
    let file = File::open(&filename).expect("unable to open file");
    let reader = BufReader::new(file);
    let mut parser = Parser::new(reader, &filename).expect("read error");
    match parser.parse() {
        Ok(ast::Class { name, .. }) => println!("Parsed {} from {}", name, filename),
        Err(e) => eprintln!("Error parsing {}: {:?}", filename, e),
    }
}
