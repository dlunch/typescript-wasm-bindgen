use swc_common::BytePos;
use swc_ecma_ast::{
    Accessibility, ClassDecl, ClassMember, Decl, ExportDecl, FnDecl, ModuleDecl, ModuleItem, Param, ParamOrTsParamProp, Pat, TsKeywordTypeKind,
    TsType, TsTypeAnn,
};
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
        _ => quote! { JsValue }, // TODO
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

fn to_rust_param(param: &Param) -> TokenStream {
    match &param.pat {
        Pat::Ident(x) => {
            let name = Ident::new(&x.sym.to_string(), Span::call_site());
            let rust_type = to_rust_type(&x.type_ann.as_ref().unwrap());

            quote! { #name: #rust_type }
        }
        _ => panic!(format!("{:?}", param)),
    }
}

fn to_rust_params<'a, T: Iterator<Item = &'a Param>>(params: T) -> impl ToTokens {
    params.map(to_rust_param).collect::<Punctuated<TokenStream, Token![,]>>()
}

fn to_rust_fn(decl: &FnDecl) -> TokenStream {
    let name = Ident::new(&decl.ident.sym.to_string(), Span::call_site());
    let return_type = to_rust_return_type(&decl.function.return_type.as_ref().unwrap());

    let params = to_rust_params(decl.function.params.iter());

    quote! { fn #name(#params) #return_type; }
}

fn to_rust_class_member(class_name: &Ident, member: &ClassMember) -> Option<TokenStream> {
    match &member {
        ClassMember::Constructor(x) => {
            let params = to_rust_params(x.params.iter().map(|x| {
                if let ParamOrTsParamProp::Param(x) = x {
                    x
                } else {
                    panic!(format!("{:?}", x))
                }
            }));

            Some(quote! {
                #[wasm_bindgen(constructor)]
                fn new(#params) -> #class_name;
            })
        }
        ClassMember::ClassProp(x) => match x.accessibility.unwrap() {
            Accessibility::Public => panic!(format!("{:?}", member)),
            _ => None,
        },
        _ => panic!(format!("{:?}", member)),
    }
}

fn to_rust_class(decl: &ClassDecl) -> TokenStream {
    let name = Ident::new(&decl.ident.sym.to_string(), Span::call_site());

    let body = decl
        .class
        .body
        .iter()
        .filter_map(|x| to_rust_class_member(&name, x))
        .collect::<TokenStream>();

    quote! {
        type #name;

        #body
    }
}

fn generate_export_decl(decl: &ExportDecl) -> TokenStream {
    match &decl.decl {
        Decl::Fn(x) => to_rust_fn(x),
        Decl::Class(x) => to_rust_class(x),
        _ => panic!(format!("{:?}", decl)),
    }
}

fn generate_module_decl(decl: &ModuleDecl) -> Option<TokenStream> {
    match &decl {
        ModuleDecl::ExportDecl(x) => Some(generate_export_decl(x)),
        ModuleDecl::Import(_) => None, // TODO Make an option to handle imports
        _ => panic!(format!("{:?}", decl)),
    }
}

fn generate_module_item(item: &ModuleItem) -> Option<TokenStream> {
    match &item {
        ModuleItem::ModuleDecl(x) => generate_module_decl(x),
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

    macro_rules! assert_tokenstream_eq {
        ($left: ident, $right: ident) => {
            assert_eq!(format!("{}", $left), format!("{}", $right))
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

            assert_tokenstream_eq!(generated, expected);
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
    fn test_complex_type() {
        let ts = "export function test(a?: null | { [index: string]: string; }): string;";
        let expected = quote! { fn test(a: JsValue) -> &str; };

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
            constructor(test: string) {}
        };";
        let expected = quote! {
            type test;

            #[wasm_bindgen(constructor)]
            fn new(test: &str) -> test;
        };

        assert_codegen_eq!(ts, expected);
    }
}
