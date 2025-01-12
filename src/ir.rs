use crate::Image;
use chumsky::prelude::*;
use std::collections::HashMap;


#[derive(Debug, Clone)]
enum Value {
    String(String),
    Bytes(Vec<u8>),
    Word(u64),
    StaticAccess(String),
    Number(i64),
    Byte(u8),
    SignedWord(i64)
}


impl Value {
    fn cast(&self, tp : &str) -> Value {
        if tp == "word" {
            if let Self::Number(n) = self {
                return Value::Word(*n as u64);
            }
            else if let Self::StaticAccess(_) = self {
                return self.clone(); // static accesses are unsigned words
            }
        }
        if tp == "bytes" {
            if let Self::String(s) = self {
                return Value::Bytes(s.as_bytes().to_vec());
            }
        }
        if tp == "byte" {
            if let Self::Number(n) = self {
                return Value::Byte(*n as u8);
            }
        }
        if tp == "signedword" {
            if let Self::Number(n) = self {
                return Value::SignedWord(*n as i64);
            }
            else if let Self::StaticAccess(_) = self {
                return self.clone(); // static accesses are unsigned words - signed works too!
            }
        }
        panic!("improper cast {:?} to {}", self, tp);
    }

    fn dump_into(&self, f_tbl : &HashMap<String, i64>, s_tbl : &HashMap<String, i64>, out : &mut Vec<u8>) {
        match self {
            Value::Bytes(v) => {
                out.extend_from_slice(&v);
            },
            Value::Word(v) => {
                out.extend_from_slice(&v.to_be_bytes());
            }
            Value::StaticAccess(s) => {
                let ptr = if let Some(p) = s_tbl.get(s) { *p } else {
                    f_tbl[s]
                };
                out.extend_from_slice(&ptr.to_be_bytes());
            }
            Value::Byte(b) => {
                out.extend_from_slice(&b.to_be_bytes());
            }
            Value::SignedWord(w) => {
                out.extend_from_slice(&w.to_be_bytes());
            }
            _ => {
                panic!("cannot dump {:?} into a vec<u8> as it is an unsupported type (did you perform correct casts?)", self);
            }
        }
    }
}


#[derive(Debug)]
struct Operation(String, Vec<Value>);


impl Operation {
    fn dump_into(&self, f_tbl : &HashMap<String, i64>, s_tbl : &HashMap<String, i64>, out : &mut Vec<u8>) {
        let Operation(name, operations) = self;
        match name.as_str() {
            "pushvl" => {
                out.push(0);
                operations[0].cast("word").dump_into(f_tbl, s_tbl, out);
            },
            "movml" => {
                out.push(16);
                operations[0].cast("signedword").dump_into(f_tbl, s_tbl, out);
                operations[1].cast("byte").dump_into(f_tbl, s_tbl, out);
            },
            "movrl" => {
                out.push(20);
                operations[0].cast("signedword").dump_into(f_tbl, s_tbl, out);
                operations[1].cast("byte").dump_into(f_tbl, s_tbl, out);
            },
            "invokevirtual" => {
                out.push(67);
                operations[0].cast("signedword").dump_into(f_tbl, s_tbl, out);
            },
            "popl" => {
                out.push(8);
                operations[0].cast("byte").dump_into(f_tbl, s_tbl, out);
            },
            "ret" => {
                out.push(66);
            },
            "dock" => {
                out.push(68);
                operations[0].cast("signedword").dump_into(f_tbl, s_tbl, out);
            },
            "loadfun" => {
                out.push(69);
                operations[0].cast("signedword").dump_into(f_tbl, s_tbl, out);
            },
            "swapl" => {
                out.push(4);
                operations[0].cast("signedword").dump_into(f_tbl, s_tbl, out);
                operations[1].cast("signedword").dump_into(f_tbl, s_tbl, out);
            },
            "call" => {
                out.push(65);
                operations[0].cast("signedword").dump_into(f_tbl, s_tbl, out);
            },
            "exit" => {
                out.push(70);
            },
            _ => {
                panic!("invalid instruction {}", name);
            }
        }
    }
}


#[derive(Debug)]
enum AstNode {
    StaticDefinition(String, Value, bool), // the last bool is whether or not this should be made public or not (listed in the table at the start of the file)
    FunctionDefinition(String, Vec<Operation>, bool) // ditto
}


fn parser() -> impl Parser<char, Vec<AstNode>, Error=Simple<char>> {
    let esc = just('\\').ignored().then(choice((just('\\'), just('n'), just('0')))).map(|(_, c)| match c {
        'n' => '\n',
        '0' => '\0',
        _ => c
    }).or(none_of('"'));
    let string = just('"').ignore_then(esc.repeated()).then_ignore(just('"')).padded().collect::<String>().map(Value::String);
    let number = just('-').ignored().then(text::int(10)).padded().map(|(_, i)| Value::Number(i.parse::<i64>().unwrap() * -1)).or(text::int(10).padded().map(|n : String| Value::Number(n.parse::<i64>().unwrap())));
    let var_access = just('$').then(text::ident()).padded().map(|(_, var)| { Value::StaticAccess(var) });
    let value = choice((string, number, var_access));
    let comment = just(';').padded().then(none_of("\n").repeated());
    let operation = text::ident().padded().then(value.clone().repeated()).then_ignore(comment.clone().repeated()).map(|(op, values)| {
        Operation(op, values)
    });
    let static_assign = just('=').ignored().then(text::ident()).padded().then(text::ident()).padded().then(value.clone()).padded().map(|(((_, name), tp), value)| { AstNode::StaticDefinition(name, value.cast(&tp), false) });
    let fndef = just('.').ignored().then(text::ident()).then_ignore(just(' ').repeated()).then(text::ident().repeated().at_most(1)).padded().then(operation.repeated()).map(|(((_, name), modifier), program)| {
        AstNode::FunctionDefinition(name, program, if modifier.len() > 0 { modifier[0] == "export" } else { false })
    });
    choice((static_assign, fndef)).padded().then_ignore(comment.repeated()).padded().repeated().then_ignore(end())
}


pub fn build(program : &str) -> Image {
    let irast = parser().parse(program).unwrap();
    let mut public_fn_table = HashMap::new();
    let public_static_table = HashMap::new();
    let mut fn_table : HashMap<String, i64> = HashMap::new();
    let mut text_section = Vec::new();
    let mut static_table : HashMap<String, i64> = HashMap::new();
    let mut static_section = Vec::new();
    for statement in &irast { // build a static table and static section
        if let AstNode::StaticDefinition(name, value, _) = statement {
            static_table.insert(name.clone(), static_section.len() as i64);
            value.dump_into(&fn_table, &static_table, &mut static_section);
        }
    }
    for statement in &irast {
        if let AstNode::FunctionDefinition(name, program, exposed) = statement {
            if *exposed {
                public_fn_table.insert(name.clone(), text_section.len() as i64);
            }
            fn_table.insert(name.clone(), (static_section.len() + text_section.len()) as i64);
            for op in program {
                op.dump_into(&fn_table, &static_table, &mut text_section);
            }
        }
    }
    println!("got final ftable {:?} (full {:?})", public_fn_table, fn_table);
    Image {
        function_table : public_fn_table,
        static_table : public_static_table,
        static_section,
        text_section
    }
}