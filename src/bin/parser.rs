extern crate som;

use som::compiler::Parser;
use std::env;
use std::fs::File;
use std::io::BufReader;

fn main() {
    let filename = env::args().nth(1).expect("filename to lex");
    let file = File::open(&filename).expect("unable to open file");
    let reader = BufReader::new(file);
    let mut parser = Parser::new(reader, filename).expect("read error");
    println!("{:#?}", parser.parse());
}
