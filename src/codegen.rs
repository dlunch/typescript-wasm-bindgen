use swc_common::BytePos;
use swc_ecma_ast::{ClassDecl, ClassMember, Decl, ExportDecl, FnDecl, ModuleDecl, ModuleItem, Param, Pat, TsKeywordTypeKind, TsType, TsTypeAnn};
use swc_ecma_parser::{lexer::Lexer, Parser, StringInput, Syntax, TsConfig};

use proc_macro2::{Ident, Span, TokenStream};
use quote::{quote, ToTokens};
use syn::{punctuated::Punctuated, Token};

fn to_rust_type(ts_type: &TsTypeAnn) -> TokenStream {
    match &*ts_type.type_ann {
        TsType::TsKeywordType(x) => match x.kind {
            TsKeywordTypeKind::TsVoidKeyword => quote! { () },
            TsKeywordTypeKind::TsNumberKeyword => quote! { f64 },
            TsKeywordTypeKind::TsStringKeyword => quote! { &str },
            TsKeywordTypeKind::TsBooleanKeyword => quote! { bool },
            _ => panic!(format!("{:?}", ts_type)),
        },
        _ => panic!(format!("{:?}", ts_type)),
    }
}

fn to_rust_return_type(ts_type: &TsTypeAnn) -> TokenStream {
    let return_type = to_rust_type(ts_type);

    match &*ts_type.type_ann {
        TsType::TsKeywordType(x) => {
            if x.kind == TsKeywordTypeKind::TsVoidKeyword {
                TokenStream::new()
            } else {
                quote! { -> #return_type }
            }
        }
        _ => quote! { -> #return_type },
    }
}

fn generate_param(param: &Param) -> TokenStream {
    match &param.pat {
        Pat::Ident(x) => {
            let name = Ident::new(&x.sym.to_string(), Span::call_site());
            let rust_type = to_rust_type(&x.type_ann.as_ref().unwrap());

            quote! { #name: #rust_type }
        }
        _ => panic!(format!("{:?}", param)),
    }
}

fn generate_params(params: &[Param]) -> impl ToTokens {
    params.iter().map(generate_param).collect::<Punctuated<TokenStream, Token![,]>>()
}

fn generate_fn_decl(decl: &FnDecl) -> TokenStream {
    let name = Ident::new(&decl.ident.sym.to_string(), Span::call_site());
    let return_type = to_rust_return_type(&decl.function.return_type.as_ref().unwrap());

    let params = generate_params(&decl.function.params);

    quote! { fn #name(#params) #return_type; }
}

fn generate_class_member(class_name: &Ident, member: &ClassMember) -> Option<TokenStream> {
    match &member {
        ClassMember::Constructor(_) => Some(quote! {
            #[wasm_bindgen(constructor)]
            fn new() -> #class_name;
        }),
        _ => panic!(format!("{:?}", member)),
    }
}

fn generate_class_decl(decl: &ClassDecl) -> TokenStream {
    eprintln!("{:?}", decl);

    let name = Ident::new(&decl.ident.sym.to_string(), Span::call_site());

    let body = decl
        .class
        .body
        .iter()
        .filter_map(|x| generate_class_member(&name, x))
        .collect::<TokenStream>();

    quote! {
        type #name;

        #body
    }
}

fn generate_export_decl(decl: &ExportDecl) -> TokenStream {
    match &decl.decl {
        Decl::Fn(x) => generate_fn_decl(x),
        Decl::Class(x) => generate_class_decl(x),
        _ => panic!(format!("{:?}", decl)),
    }
}

fn generate_module_decl(decl: &ModuleDecl) -> TokenStream {
    match &decl {
        ModuleDecl::ExportDecl(x) => generate_export_decl(x),
        _ => panic!(format!("{:?}", decl)),
    }
}

fn generate_module_item(item: &ModuleItem) -> Option<TokenStream> {
    match &item {
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

    let definitions = module.body.iter().filter_map(generate_module_item).collect::<TokenStream>();

    quote! {
        #[wasm_bindgen(module = #module_name)]
        extern "C" {
            #definitions
        }
    }
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
    fn test_function_params() {
        let ts = "export function test(a: number, b: boolean, c: string): void;";
        let expected = quote! { fn test(a: f64, b: bool, c: &str); };

        assert_codegen_eq!(ts, expected);
    }

    #[test]
    fn test_class() {
        let ts = "export class test {};";
        let expected = quote! { type test; };

        assert_codegen_eq!(ts, expected);
    }

    #[test]
    fn test_class_constructor() {
        let ts = "export class test {
            constructor() {}
        };";
        let expected = quote! {
            type test;

            #[wasm_bindgen(constructor)]
            fn new() -> test;
        };

        assert_codegen_eq!(ts, expected);
    }
}
