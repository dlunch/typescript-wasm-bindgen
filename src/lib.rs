mod codegen;

use proc_macro::TokenStream;
use syn::{parse_macro_input, punctuated::Punctuated, LitStr, Token};

#[proc_macro]
pub fn typescript(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input with Punctuated::<LitStr, Token![,]>::parse_separated_nonempty);
    let (filename, module_name) = (&input[0], &input[1]);

    codegen::generate_wasm_bindgen_bindings(&filename.value(), &module_name.value()).into()
}
