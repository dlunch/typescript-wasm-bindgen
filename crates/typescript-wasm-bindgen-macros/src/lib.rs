use std::fs;
use std::str;

use proc_macro::TokenStream;
use syn::{parse_macro_input, punctuated::Punctuated, LitStr, Token};

use typescript_wasm_bindgen_codegen::generate_wasm_bindgen_bindings;

#[proc_macro]
pub fn typescript(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input with Punctuated::<LitStr, Token![,]>::parse_separated_nonempty);
    let (filename, module_name) = (&input[0], &input[1]);

    let file_path = std::env::current_dir().unwrap().join(&filename.value());
    let content = fs::read(file_path).unwrap();
    let content_str = str::from_utf8(&content).unwrap();

    generate_wasm_bindgen_bindings(content_str, &module_name.value()).into()
}
