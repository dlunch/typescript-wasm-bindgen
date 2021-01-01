use std::fs;
use std::str;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, LitStr};

#[proc_macro]
pub fn typescript(input: TokenStream) -> TokenStream {
    let filename = parse_macro_input!(input as LitStr).value();

    let file_path = std::env::current_dir().unwrap().join(&filename);
    let file = fs::read(file_path).unwrap();

    let content = str::from_utf8(&file).unwrap();
    eprintln!("{}", content);

    (quote! {
        "TEST"
    })
    .into()
}
