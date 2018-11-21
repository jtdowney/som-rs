extern crate som;

use som::compiler::Lexer;
use std::env;
use std::fs::File;
use std::io::BufReader;

#[cfg_attr(tarpaulin, skip)]
fn main() {
    let filename = env::args().nth(1).expect("filename to lex");
    let file = File::open(filename).expect("unable to open file");
    let reader = BufReader::new(file);
    let lexer = Lexer::new(reader).expect("read error");
    let tokens = lexer.collect::<Result<Vec<_>, _>>().expect("tokens");
    println!("{:#?}", tokens);
}
