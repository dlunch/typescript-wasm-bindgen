use std::fs;
use std::str;

use proc_macro::TokenStream;
use quote::quote;
use swc_common::BytePos;
use swc_ecma_parser::{lexer::Lexer, Parser, StringInput, Syntax};
use syn::{parse_macro_input, LitStr};

#[proc_macro]
pub fn typescript(input: TokenStream) -> TokenStream {
    let filename = parse_macro_input!(input as LitStr).value();

    let file_path = std::env::current_dir().unwrap().join(&filename);
    let file = fs::read(file_path).unwrap();

    let content = str::from_utf8(&file).unwrap();
    let lexer = Lexer::new(
        Syntax::Typescript(Default::default()),
        Default::default(),
        StringInput::new(content, BytePos(0), BytePos(content.len() as u32)),
        None,
    );

    let mut parser = Parser::new_from(lexer);
    let module = parser.parse_typescript_module().unwrap();

    eprintln!("{:?}", module.body[0]);

    (quote! {
        "TEST"
    })
    .into()
}
