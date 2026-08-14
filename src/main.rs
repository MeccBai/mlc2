use crate::lexer::tokenize;

mod ast;
mod error;
mod lexer;
mod parser;


fn main() {
    let code = r#"
        import std::io;
        import std::generic;

        generic type1 {
            std::generic::is_integer;
        };

        generic type2 {
            pub func add(self, i8 param1) -> i8;
        };

        unit a<type1> {
            i8 x;
            type1$b;

            pub func add (self, i8 param1) -> i8 {
                return self->x + param1;
            }
        };

        func<type1> test(type1 a) -> i8 {
            return a + 1;
        }

        using u8_unit = a<u8>;

        func test1(u8 a,u8 b ,u8 c) -> i8 {
            var d = a + b + c;
            return d;
        }

        func test2(u8 a,u8 b) -> i8 {
            var d = a + b;
            return d;
        }

        uint bx {
            i8 xx;
            pub func add (self, i8 param1) -> i8 {
                return self->xx + param1;
            }
        };

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

            var sum = { 5, {1,2,3} |> test1 } |> test2;

            bx c;
            var result = 10 |> c.add;

            std::io::print("Hello, world!");
        }
    "#;

    let err_h = error::ErrorHandle::new("test".to_string());

    let tokens = tokenize(code).unwrap_or_else(|e| {
        err_h.token_error(e, code);
        std::process::exit(1);
    });

    for x in tokens.iter() {
        println!("{:?}", x);
    }
}
