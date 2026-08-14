use crate::lexer::TokenPack;
use crate::lexer::token::Token;
use crate::parser::split::{get_name, next_and_assert, split_by_semicolon};
use crate::parser::{GenericDecl, TempGlobalStmt, TokenIter};

fn generic_process(iter: &mut TokenIter) -> Result<TempGlobalStmt, TokenPack> {
    next_and_assert(iter, Token::Generic)?;
    let name = get_name(iter)?;
    next_and_assert(iter, Token::LBrace)?;
    let requires = split_by_semicolon(iter, Token::RBrace)?;
    Ok(TempGlobalStmt::Generic(GenericDecl { name, requires }))
}

#[test]
fn test_generic_process() {
    use crate::lexer;

    let code = r#"
        generic type1 {
            std::requires::is_integer;
        };
    "#;

    let code2 = r#"
        generic type2 {
            pub func add(self, i8 param1) -> i8;
        };
    "#;

    let tokens1 = lexer::tokenize(code).unwrap();

    let tokens2 = lexer::tokenize(code2).unwrap();

    let generic_print = |generic: &GenericDecl| {
        println!("Parsed generic: {:?}", generic.name);
        for req in &generic.requires {
            print!("{:?}", req);
        }
        println!();
    };

    let generic_match = |result: Result<TempGlobalStmt, TokenPack>| match result {
        Ok(generic) => {
            if let TempGlobalStmt::Generic(g) = generic {
                generic_print(&g);
            } else {
                println!("Parsed statement is not a Generic");
            }
        }
        Err(e) => {
            eprintln!("Error parsing generic: {:?}", e);
            panic!("Failed to parse generic");
        }
    };

    generic_match(generic_process(&mut tokens1.into_iter()));

    generic_match(generic_process(&mut tokens2.into_iter()));
    
    println!()
}
