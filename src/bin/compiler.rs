extern crate som;

use som::compiler;
use std::env;

fn main() {
    let filename = env::args().nth(1).expect("filename to compile");
    let class = compiler::compile_path(filename).expect("class to compile");
    println!("{:#?}", class);
}
