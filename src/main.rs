use crate::ast::config::Config;
use crate::ast::types::CompileType::Generic;
use crate::lexer::tokenize;

mod ast;
pub mod build;
mod error;
mod lexer;
mod parser;

fn main() {
    let code = r#"
        import std::io;
        import std::generic;

        generic type1 {
            std::generic::is_integer;
            std::generic::max_bits<16>;
        };

        generic type2 {
            pub func add(self, param1:i8) -> i8;
        };

        unit a<T:type1> {
            x:i8;
            b:$T;
        };

        a::func add(self, param1:i8) -> i8 {
            return self->x + param1;
        }

        func<T:type1> test(a:T) -> i8 {
            return a + 1;
        }

        using u8_unit = a<u8>;

        export func test1(a:u8,b:u8,c:u8) -> i8 {
            var d = a + b + c;
            return d;
        }

        export func test2(a:u8,b:u8) -> i8 {
            var d = a + b;
            return d;
        }

        func main() -> i8 {
            // 这是一个注释
            var cc = u8_unit {0,null};
            match (1) {
                1 => {
                    cc->x = 1;
                },
                2 => {
                    cc->x = 2;
                },
                _ => {
                    cc->x = 3;
                }
            }

            var sum =  {
                5,
                {1,2,3} |> test1
            } |> test2 ;


            std::io::print("Hello, world!");
        }
    "#;

    let err_h = error::ErrorHandle::new("test".to_string());

    let lexed = tokenize(code).unwrap_or_else(|e| {
        err_h.token_error(e, code);
        std::process::exit(1);
    });

    let (module, errors) = parser::parse(&lexed.tokens, code.len());
    if !errors.is_empty() {
        for error in &errors {
            err_h.parse_error(error, code);
        }
        return;
    }
    let Some(module) = module else {
        return;
    };

    let config = Config::new(vec!["".to_string()], "".to_string(), "".to_string(), err_h);

    let ast = ast::AbstractSyntaxTree::new(config, module);

    //if let Some(module) = module {
    //    for (item, _) in &module {
    //        println!("{}", item.dump());
    //    }
    //}
}
