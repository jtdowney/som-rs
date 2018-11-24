extern crate som;

use som::compiler::Parser;
use std::env;
use std::fs::File;
use std::io::BufReader;

#[cfg_attr(tarpaulin, skip)]
fn main() {
    let filename = env::args().nth(1).expect("filename to parse");
    let file = File::open(&filename).expect("unable to open file");
    let reader = BufReader::new(file);
    let mut parser = Parser::new(reader, &filename);
    let class = parser.parse().expect("parser error");
    println!("{:#?}", class);
}
