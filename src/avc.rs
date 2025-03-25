// AnyVm C
// shitty C dialect for writing anyvm code without using the IR
// it is literally just a slightly nicer way to write anyvm ir. for instance; functions are no more complex than un-type-checked `long`s floating in space.
use crate::Image;
use chumsky::prelude::*;
use std::collections::HashMap;


#[derive(Debug, Clone)]
enum Type {
    Long,
    Char,
    Ref(Box<Type>)
}


impl Type {
    fn from_str(thing : &str) -> Type {
        match thing {
            "long" => Type::Long,
            _ => panic!("invalid type {}", thing) // TODO: error handling
        }
    }
}


#[derive(Debug, Clone)]
struct Variable {
    name : String,
    t : Type,
    v : Option<Expression>
}


#[derive(Debug, Clone)]
enum Command {
    FunctionCall(String, Vec<Expression>)
}


#[derive(Debug)]
enum TopLevel {
    StaticDefinition(Variable),
    Export(String),
    ExportFn(String)
}


fn type_parser() -> impl Parser<char, Type, Error=Simple<char>> {
    just('&').repeated().then(text::ident()).padded().map(|(refs, t)| {
        let mut tp = Type::from_str(&t);
        for _ in 0..refs.len() {
            tp = Type::Ref(Box::new(tp));
        }
        tp
    })
}


#[derive(Debug, Clone)]
enum Expression {
    Number(i64),
    NtString(String), // null terminated string
    Function(Vec<Variable>, Vec<Command>), // arguments, functions
    Sref(i64), // reference to a location in the static table
}


fn string_parse() -> impl Parser<char, String, Error=Simple<char>> {
    let esc = just('\\').ignored().then(choice((just('\\'), just('n'), just('0')))).map(|(_, c)| match c {
        'n' => '\n',
        '0' => '\0',
        _ => c
    }).or(none_of('"'));
    just('"').ignore_then(esc.repeated()).then_ignore(just('"')).padded().collect::<String>()
}


fn atom() -> impl Parser<char, Expression, Error=Simple<char>> {
    text::int(10).map(|s: String| {
        Expression::Number(s.parse().unwrap())
    }).or(string_parse().map(Expression::NtString))
}


fn expression_parser() -> impl Parser<char, Expression, Error=Simple<char>> {
    recursive(|expression_parser| {
        let command_parser = text::ident().padded().then(expression_parser.separated_by(just(',')).allow_trailing().delimited_by(just('('), just(')')).collect::<Vec<_>>()).map(|(name, args)| {
            Command::FunctionCall(name, args)
        });
        let arg_tuple = variable_parser().separated_by(just(',')).allow_trailing().delimited_by(just('('), just(')')).collect::<Vec<_>>();
        let function = arg_tuple.or_not().padded().then(command_parser.padded().repeated().delimited_by(just('{'), just('}'))).map(|(args, commands)| {
            Expression::Function(match args {
                Some(args) => args,
                None => Vec::new()
            }, commands)
        });
        function.or(atom())
    })
}


fn variable_parser() -> impl Parser<char, Variable, Error=Simple<char>> { // parse a typed variable (this can be a function argument)
    // C-style
    type_parser().then(text::ident()).padded().map(|(vtype, name)| {
        Variable {
            name,
            t : vtype,
            v : None
        }
    })
}


fn variable_parser_with_value() -> impl Parser<char, Variable, Error=Simple<char>> { // parse a typed variable with optional `= value` after it
    variable_parser().padded().then_ignore(just('=')).padded().then(
        expression_parser().repeated().at_most(1)
    ).map(|(mut var, expr)| {
        if expr.len() == 1 {
            var.v = Some(expr[0].clone())
        }
        var
    })
}


fn parser() -> impl Parser<char, Vec<TopLevel>, Error=Simple<char>> {
    choice((
        text::keyword("export").padded().ignore_then(text::keyword("function").or_not()).padded().then(text::ident()).map(|(is_function, name)| {
            if let Some(_) = is_function {
                TopLevel::ExportFn(name)
            }
            else {
                TopLevel::Export(name)
            }
        }),
        variable_parser_with_value().map(|v| {
            TopLevel::StaticDefinition(v)
        })
    )).padded().repeated().then_ignore(end())
}


struct ImageBuilder {
    static_section : Vec<u8>,
    text_section : Vec<u8>,
    static_table : HashMap<String, i64>,
    function_table : HashMap<String, i64>,
    pub_st_table : HashMap<String, i64>,
    pub_fn_table : HashMap<String, i64>
}


impl ImageBuilder {
    fn new() -> Self {
        Self {
            static_section : Vec::new(),
            text_section : Vec::new(),
            static_table : HashMap::new(),
            function_table : HashMap::new(),
            pub_st_table : HashMap::new(),
            pub_fn_table : HashMap::new()
        }
    }

    fn build(&mut self, program : &mut Vec<TopLevel>) {
        println!("program: {:?}", program);
        for statement in program {
            statement.static_collapse(self);
        }
    }

    fn into_image(self) -> Image {
        Image {
            function_table : self.pub_fn_table,
            text_section : self.text_section,
            static_table : self.pub_st_table,
            static_section : self.static_section
        }
    }
}


impl TopLevel {
    fn static_collapse(&mut self, image : &mut ImageBuilder) { // fill a static table
        let static_pointer = image.static_section.len();
        match self {
            Self::StaticDefinition(var) => {
                if let Some(v) = &var.v {
                    v.insert_static(image);
                }
                var.v = Some(Expression::Sref(static_pointer as i64));
            },
            _ => {}
        }
    }
}


impl Expression {
    fn insert_static(&self, image : &mut ImageBuilder) {
        match self {
            Self::Number(i) => {
                image.static_section.extend(i.to_be_bytes());
            },
            Self::NtString(s) => {
                image.static_section.extend(s.as_bytes());
                image.static_section.extend(s.len().to_be_bytes());
            }
            Self::Function(_, program) => {
                
            },
            Self::Sref(to) => {}
        }
    }
}


pub fn build(program : &str) -> Image {
    let mut irast = parser().parse(r#"
    long varname = 80
    long main = {
        print("Test message!")
    }
    export function main
    "#).unwrap();

    let mut builder = ImageBuilder::new();
    builder.build(&mut irast);
    println!("nazi: {:?}", builder.static_section);
    builder.into_image()
}
