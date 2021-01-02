use swc_common::BytePos;
use swc_ecma_ast::{ClassDecl, Decl, ExportDecl, FnDecl, ModuleDecl, ModuleItem, TsKeywordTypeKind, TsType};
use swc_ecma_parser::{lexer::Lexer, Parser, StringInput, Syntax, TsConfig};

use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;

fn to_rust_type(ts_type: &TsType) -> TokenStream {
    match ts_type {
        TsType::TsKeywordType(x) => match x.kind {
            TsKeywordTypeKind::TsVoidKeyword => {
                quote! {
                    ()
                }
            }
            _ => panic!(format!("{:?}", ts_type)),
        },
        _ => panic!(format!("{:?}", ts_type)),
    }
}

fn to_rust_return_type(ts_type: &TsType) -> TokenStream {
    let return_type = to_rust_type(ts_type);

    match ts_type {
        TsType::TsKeywordType(x) => {
            if x.kind == TsKeywordTypeKind::TsVoidKeyword {
                TokenStream::new()
            } else {
                quote! {
                    -> #return_type
                }
            }
        }
        _ => {
            quote! {
                -> #return_type
            }
        }
    }
}

fn generate_fn_decl(decl: FnDecl) -> TokenStream {
    let name = Ident::new(&decl.ident.sym.to_string(), Span::call_site());
    let return_type = to_rust_return_type(&decl.function.return_type.unwrap().type_ann);

    quote! {
        fn #name() #return_type;
    }
}

fn generate_class_decl(decl: ClassDecl) -> TokenStream {
    eprintln!("{:?}", decl);

    let name = Ident::new(&decl.ident.sym.to_string(), Span::call_site());

    quote! {
        type #name;
    }
}

fn generate_export_decl(decl: ExportDecl) -> TokenStream {
    match decl.decl {
        Decl::Fn(x) => generate_fn_decl(x),
        Decl::Class(x) => generate_class_decl(x),
        _ => panic!(format!("{:?}", decl)),
    }
}

fn generate_module_decl(decl: ModuleDecl) -> TokenStream {
    match decl {
        ModuleDecl::ExportDecl(x) => generate_export_decl(x),
        _ => panic!(format!("{:?}", decl)),
    }
}

fn generate_module_item(item: ModuleItem) -> Option<TokenStream> {
    match item {
        ModuleItem::ModuleDecl(x) => Some(generate_module_decl(x)),
        ModuleItem::Stmt(_) => None,
    }
}

pub fn generate_wasm_bindgen_bindings(content: &str, module_name: &str) -> TokenStream {
    let lexer = Lexer::new(
        Syntax::Typescript(TsConfig {
            dynamic_import: true, // TODO tsconfig?
            ..Default::default()
        }),
        Default::default(),
        StringInput::new(content, BytePos(0), BytePos(content.len() as u32)),
        None,
    );

    let mut parser = Parser::new_from(lexer);
    let module = parser.parse_typescript_module().unwrap();

    let definitions = module.body.into_iter().filter_map(generate_module_item).collect::<TokenStream>();

    let result = quote! {
        #[wasm_bindgen(module = #module_name)]
        extern "C" {
            #definitions
        }
    };

    eprintln!("{}", result);

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! assert_debug_eq {
        ($left: ident, $right: ident) => {
            assert_eq!(format!("{:?}", $left), format!("{:?}", $right))
        };
    }

    macro_rules! assert_codegen_eq {
        ($ts: ident, $expected: ident) => {
            let expected = quote! {
                #[wasm_bindgen(module = "test")]
                extern "C" {
                    #$expected
                }
            };
            let generated = generate_wasm_bindgen_bindings($ts, "test");

            assert_debug_eq!(generated, expected);
        };
    }

    #[test]
    fn test_function() {
        let ts = "export function test(): void;";
        let expected = quote! { fn test(); };

        assert_codegen_eq!(ts, expected);
    }

    #[test]
    fn test_class() {
        let ts = "export class test {};";
        let expected = quote! { type test; };

        assert_codegen_eq!(ts, expected);
    }
}
