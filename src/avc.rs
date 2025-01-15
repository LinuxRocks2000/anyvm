// AnyVm C
// shitty C dialect for writing anyvm code without using the IR
use crate::Image;
use chumsky::prelude::*;
use std::collections::HashMap;

fn parser() -> impl Parser<char, Vec<AstNode>, Error=Simple<char>> {

}


pub fn build(program : &str) -> Image {
    let irast = parser().parse(program).unwrap();
    let mut public_fn_table = HashMap::new();
    let public_static_table = HashMap::new();
    let mut fn_table : HashMap<String, i64> = HashMap::new();
    let mut text_section = Vec::new();
    let mut static_table : HashMap<String, i64> = HashMap::new();
    let mut static_section = Vec::new();

    Image {
        function_table : public_fn_table,
        static_table : public_static_table,
        static_section,
        text_section
    }
}