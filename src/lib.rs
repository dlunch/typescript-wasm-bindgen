use proc_macro::TokenStream;
use syn::{parse_macro_input, LitStr};
use quote::quote;

#[proc_macro]
pub fn my_macro(input: TokenStream) -> TokenStream {
    // macro input must be `LitStr`, which is a string literal.
    // if not, a relevant error message will be generated.
    let input = parse_macro_input!(input as LitStr);

    // get value of the string literal.
    let str_value = input.value();

    // do something with value...
    let str_value = str_value.to_uppercase();

    // generate code, include `str_value` variable (automatically encodes
    // `String` as a string literal in the generated code)
    (quote!{
        #str_value
    }).into()
}
