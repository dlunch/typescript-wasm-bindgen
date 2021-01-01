mod codegen;

use proc_macro::TokenStream;
use syn::{parse::Parser, punctuated::Punctuated, LitStr, Token};

#[proc_macro]
pub fn typescript(input: TokenStream) -> TokenStream {
    let parser = Punctuated::<LitStr, Token![,]>::parse_separated_nonempty;
    let args = parser.parse(input).unwrap();
    let (filename, module_name) = (&args[0], &args[1]);

    codegen::generate_wasm_bindgen_bindings(&filename.value(), &module_name.value()).into()
}
