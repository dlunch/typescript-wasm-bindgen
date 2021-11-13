use std::collections::HashSet;

use swc_common::BytePos;
use swc_ecma_ast::{
    Accessibility, ClassDecl, ClassMember, ClassMethod, Decl, ExportDecl, FnDecl, ModuleDecl, ModuleItem, Param, ParamOrTsParamProp, Pat, PropName,
    TsEntityName, TsKeywordTypeKind, TsType,
};
use swc_ecma_parser::{lexer::Lexer, Parser, StringInput, Syntax, TsConfig};

use proc_macro2::{Ident, Span, TokenStream};
use quote::{quote, ToTokens};
use syn::{punctuated::Punctuated, Token};

struct Codegen {
    #[allow(dead_code)]
    dts: bool,
}

impl Codegen {
    pub fn generate(content: &str, module_name: &str, dts: bool) -> TokenStream {
        let lexer = Lexer::new(
            Syntax::Typescript(TsConfig {
                dynamic_import: true, // TODO tsconfig?
                dts,
                ..Default::default()
            }),
            Default::default(),
            StringInput::new(content, BytePos(0), BytePos(content.len() as u32)),
            None,
        );

        let mut parser = Parser::new_from(lexer);
        let module = parser.parse_typescript_module().unwrap();

        let codegen = Self { dts };
        let definitions = module
            .body
            .iter()
            .filter_map(|x| codegen.generate_module_item(x))
            .collect::<TokenStream>();

        quote! {
            #[wasm_bindgen(module = #module_name)]
            extern "C" {
                #definitions
            }
        }
    }

    fn to_rust_type(&self, ts_type: &TsType) -> TokenStream {
        match ts_type {
            TsType::TsKeywordType(x) => match x.kind {
                TsKeywordTypeKind::TsVoidKeyword => quote! { () },
                TsKeywordTypeKind::TsNumberKeyword => quote! { f64 },
                TsKeywordTypeKind::TsStringKeyword => quote! { &str },
                TsKeywordTypeKind::TsBooleanKeyword => quote! { bool },
                _ => panic!("unhandled {:?}", ts_type),
            },
            _ => quote! { JsValue }, // TODO
        }
    }

    fn to_rust_return_type(&self, ts_type: Option<&TsType>) -> TokenStream {
        if let Some(ts_type) = ts_type {
            let rust_type = self.to_rust_type(ts_type);

            match ts_type {
                TsType::TsKeywordType(x) => {
                    if x.kind == TsKeywordTypeKind::TsVoidKeyword {
                        TokenStream::new()
                    } else if x.kind == TsKeywordTypeKind::TsStringKeyword {
                        quote! { -> String }
                    } else {
                        quote! { -> #rust_type }
                    }
                }
                _ => quote! { -> #rust_type },
            }
        } else {
            TokenStream::new()
        }
    }

    fn extract_promise_inner_type<'a>(&self, ts_type: Option<&'a TsType>) -> Option<&'a TsType> {
        match ts_type {
            Some(TsType::TsTypeRef(type_ref)) => {
                if let TsEntityName::Ident(ident) = &type_ref.type_name {
                    if ident.sym.to_string() == "Promise" {
                        Some(&type_ref.type_params.as_ref().unwrap().params[0])
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn to_rust_param(&self, param: &Param) -> TokenStream {
        match &param.pat {
            Pat::Ident(x) => {
                let name = Ident::new(&x.id.sym.to_string(), Span::call_site());
                let rust_type = self.to_rust_type(&x.type_ann.as_ref().unwrap().type_ann);

                quote! { #name: #rust_type }
            }
            _ => panic!("unhandled {:?}", param),
        }
    }

    fn to_rust_params<'a, T: Iterator<Item = &'a Param>>(&self, params: T) -> impl ToTokens {
        params.map(|x| self.to_rust_param(x)).collect::<Punctuated<TokenStream, Token![,]>>()
    }

    fn to_rust_fn(&self, fn_decl: &FnDecl) -> TokenStream {
        let name = Ident::new(&fn_decl.ident.sym.to_string(), Span::call_site());

        let return_type = fn_decl.function.return_type.as_ref().map(|x| &*x.type_ann);
        let params = self.to_rust_params(fn_decl.function.params.iter());

        if let Some(return_type) = self.extract_promise_inner_type(return_type) {
            let return_type = self.to_rust_return_type(Some(return_type));

            quote! { pub async fn #name(#params) #return_type; }
        } else {
            let return_type = self.to_rust_return_type(return_type);

            quote! { pub fn #name(#params) #return_type; }
        }
    }

    fn to_rust_class_method(&self, class_method: &ClassMethod, class: &ClassDecl) -> Option<TokenStream> {
        let class_name = Ident::new(&class.ident.sym.to_string(), Span::call_site());

        if class_method.accessibility.is_none() || class_method.accessibility.unwrap() == Accessibility::Public {
            let params = if !class_method.function.params.is_empty() {
                let params = self.to_rust_params(class_method.function.params.iter());

                quote! {
                    this: &#class_name, #params
                }
            } else {
                quote! {
                    this: &#class_name
                }
            };

            let (name, original_name) = self.to_rust_class_method_name(class_method, class);
            let name_ident = Ident::new(&name, Span::call_site());

            let ts_type = class_method.function.return_type.as_ref().map(|x| &*x.type_ann);
            if let Some(return_type) = self.extract_promise_inner_type(ts_type) {
                let return_type = self.to_rust_return_type(Some(return_type));

                Some(quote! {
                    #[wasm_bindgen(method, js_name = #original_name)]
                    pub async fn #name_ident(#params) #return_type;
                })
            } else {
                let return_type = self.to_rust_return_type(ts_type);

                Some(quote! {
                    #[wasm_bindgen(method, js_name = #original_name)]
                    pub fn #name_ident(#params) #return_type;
                })
            }
        } else {
            None
        }
    }

    fn extract_prop_name(&self, prop_name: &PropName) -> String {
        if let PropName::Ident(ident) = &prop_name {
            ident.sym.to_string()
        } else {
            panic!("unhandled {:?}", prop_name)
        }
    }

    fn extract_pat(&self, pat: &Pat) -> String {
        if let Pat::Ident(ident) = pat {
            ident.id.sym.to_string()
        } else {
            panic!("unhandled {:?}", pat)
        }
    }

    fn to_rust_class_method_name(&self, method: &ClassMethod, class: &ClassDecl) -> (String, String) {
        let original_name = self.extract_prop_name(&method.key);

        let overloads = class.class.body.iter().filter_map(|x| match x {
            ClassMember::Method(x) => {
                let overload_name = self.extract_prop_name(&x.key);
                if overload_name == original_name {
                    Some(x)
                } else {
                    None
                }
            }
            _ => None,
        });

        let base_overload = overloads.min_by_key(|&x| x.function.params.len());

        if let Some(base_overload) = base_overload {
            // find base overload and append `with_{param}_and_{param}`..

            let my_params = method.function.params.iter().map(|x| self.extract_pat(&x.pat));
            let base_params = base_overload
                .function
                .params
                .iter()
                .map(|x| self.extract_pat(&x.pat))
                .collect::<HashSet<_>>();

            let added_params = my_params.filter(|x| !base_params.contains(x)).collect::<Vec<_>>();

            if !added_params.is_empty() {
                let name = format!("{}_with_{}", original_name, added_params.join("_and_"));

                return (name, original_name);
            }
        }
        (original_name.to_owned(), original_name)
    }

    fn to_rust_class_member(&self, class: &ClassDecl, member: &ClassMember) -> Option<TokenStream> {
        let class_name = Ident::new(&class.ident.sym.to_string(), Span::call_site());

        match &member {
            ClassMember::Constructor(x) => {
                let params = self.to_rust_params(x.params.iter().map(|x| {
                    if let ParamOrTsParamProp::Param(x) = x {
                        x
                    } else {
                        panic!("unhandled {:?}", x)
                    }
                }));

                Some(quote! {
                    #[wasm_bindgen(constructor)]
                    pub fn new(#params) -> #class_name;
                })
            }
            ClassMember::ClassProp(_) => None,
            ClassMember::Method(x) => self.to_rust_class_method(x, class),
            _ => panic!("unhandled {:?}", member),
        }
    }

    fn to_rust_class(&self, class: &ClassDecl) -> TokenStream {
        let name = Ident::new(&class.ident.sym.to_string(), Span::call_site());

        let body = class
            .class
            .body
            .iter()
            .filter_map(|x| self.to_rust_class_member(class, x))
            .collect::<TokenStream>();

        let constructor = if !class.class.body.iter().any(|x| x.is_constructor()) {
            quote! {
                #[wasm_bindgen(constructor)]
                pub fn new() -> #name;
            }
        } else {
            quote! {}
        };

        quote! {
            pub type #name;

            #body

            #constructor
        }
    }

    fn generate_export_decl(&self, export: &ExportDecl) -> Option<TokenStream> {
        match &export.decl {
            Decl::Fn(x) => Some(self.to_rust_fn(x)),
            Decl::Class(x) => Some(self.to_rust_class(x)),
            Decl::TsModule(_) => {
                // TODO
                None
            }
            _ => panic!("unhandled {:?}", export),
        }
    }

    fn generate_module_decl(&self, module: &ModuleDecl) -> Option<TokenStream> {
        match &module {
            ModuleDecl::ExportDecl(x) => self.generate_export_decl(x),
            ModuleDecl::Import(_) => None, // TODO Make an option to handle imports
            _ => panic!("unhandled {:?}", module),
        }
    }

    fn generate_module_item(&self, item: &ModuleItem) -> Option<TokenStream> {
        match &item {
            ModuleItem::ModuleDecl(x) => self.generate_module_decl(x),
            ModuleItem::Stmt(_) => None,
        }
    }
}

pub fn generate_wasm_bindgen_bindings(content: &str, module_name: &str, dts: bool) -> TokenStream {
    Codegen::generate(content, module_name, dts)
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
            let generated = generate_wasm_bindgen_bindings($ts, "test", false);

            assert_tokenstream_eq!(generated, expected);
        };
    }

    #[test]
    fn test_function() {
        let ts = "export function test(): void;";
        let expected = quote! { pub fn test(); };

        assert_codegen_eq!(ts, expected);
    }

    #[test]
    fn test_function_params() {
        let ts = "export function test(a: number, b: boolean, c: string): void;";
        let expected = quote! { pub fn test(a: f64, b: bool, c: &str); };

        assert_codegen_eq!(ts, expected);
    }

    #[test]
    fn test_complex_type() {
        let ts = "export function test(a?: null | { [index: string]: string; }): string;";
        let expected = quote! { pub fn test(a: JsValue) -> String; };

        assert_codegen_eq!(ts, expected);
    }

    #[test]
    fn test_class() {
        let ts = "export class test {};";
        let expected = quote! {
            pub type test;

            #[wasm_bindgen(constructor)]
            pub fn new() -> test;
        };

        assert_codegen_eq!(ts, expected);
    }

    #[test]
    fn test_class_constructor() {
        let ts = "export class test {
            constructor(test: string) {}
        };";
        let expected = quote! {
            pub type test;

            #[wasm_bindgen(constructor)]
            pub fn new(test: &str) -> test;
        };

        assert_codegen_eq!(ts, expected);
    }

    #[test]
    fn test_class_method() {
        let ts = "export class test {
            constructor(test: string) {}

            test(test: number): string {}

            test1(test: number) {}
        };";
        let expected = quote! {
            pub type test;

            #[wasm_bindgen(constructor)]
            pub fn new(test: &str) -> test;

            #[wasm_bindgen(method, js_name = "test")]
            pub fn test(this: &test, test: f64) -> String;

             #[wasm_bindgen(method,  js_name = "test1")]
            pub fn test1(this: &test, test: f64);
        };

        assert_codegen_eq!(ts, expected);
    }

    #[test]
    fn test_class_method_overload() {
        let ts = "export class test {
            constructor(test: string) {}

            test(): string {}

            test(test: number): string {}

            test(test: number, test1: string) {}
        };";
        let expected = quote! {
            pub type test;

            #[wasm_bindgen(constructor)]
            pub fn new(test: &str) -> test;

            #[wasm_bindgen(method, js_name = "test")]
            pub fn test(this: &test) -> String;

            #[wasm_bindgen(method, js_name = "test")]
            pub fn test_with_test(this: &test, test: f64) -> String;

            #[wasm_bindgen(method, js_name = "test")]
            pub fn test_with_test_and_test1(this: &test, test: f64, test1: &str);
        };

        assert_codegen_eq!(ts, expected);
    }

    #[test]
    fn test_async_function() {
        let ts = "export function test(): Promise<void>;";
        let expected = quote! { pub async fn test(); };

        assert_codegen_eq!(ts, expected);
    }

    #[test]
    fn test_async_member_function() {
        let ts = "export class test {
            async test(): Promise<void>;
        };";
        let expected = quote! {
            pub type test;

            #[wasm_bindgen(method, js_name = "test")]
            pub async fn test(this: &test);

            #[wasm_bindgen(constructor)]
            pub fn new() -> test;
        };

        assert_codegen_eq!(ts, expected);
    }
}
