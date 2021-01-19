use swc_common::BytePos;
use swc_ecma_ast::{
    Accessibility, ClassDecl, ClassMember, ClassMethod, Decl, ExportDecl, FnDecl, ModuleDecl, ModuleItem, Param, ParamOrTsParamProp, Pat, PropName,
    TsKeywordTypeKind, TsType, TsTypeAnn,
};
use swc_ecma_parser::{lexer::Lexer, Parser, StringInput, Syntax, TsConfig};

use proc_macro2::{Ident, Span, TokenStream};
use quote::{quote, ToTokens};
use syn::{punctuated::Punctuated, Token};

struct Codegen {}

impl Codegen {
    pub fn generate(content: &str, module_name: &str) -> TokenStream {
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

        let codegen = Self {};
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

    fn to_rust_type(&self, ts_type: &TsTypeAnn) -> TokenStream {
        match &*ts_type.type_ann {
            TsType::TsKeywordType(x) => match x.kind {
                TsKeywordTypeKind::TsVoidKeyword => quote! { () },
                TsKeywordTypeKind::TsNumberKeyword => quote! { f64 },
                TsKeywordTypeKind::TsStringKeyword => quote! { &str },
                TsKeywordTypeKind::TsBooleanKeyword => quote! { bool },
                _ => panic!(format!("unhandled {:?}", ts_type)),
            },
            _ => quote! { JsValue }, // TODO
        }
    }

    fn to_rust_return_type(&self, ts_type: &Option<TsTypeAnn>) -> TokenStream {
        if ts_type.is_none() {
            TokenStream::new()
        } else {
            let return_type = self.to_rust_type(&ts_type.as_ref().unwrap());

            match &*ts_type.as_ref().unwrap().type_ann {
                TsType::TsKeywordType(x) => {
                    if x.kind == TsKeywordTypeKind::TsVoidKeyword {
                        TokenStream::new()
                    } else if x.kind == TsKeywordTypeKind::TsStringKeyword {
                        quote! { -> String }
                    } else {
                        quote! { -> #return_type }
                    }
                }
                _ => quote! { -> #return_type },
            }
        }
    }

    fn to_rust_param(&self, param: &Param) -> TokenStream {
        match &param.pat {
            Pat::Ident(x) => {
                let name = Ident::new(&x.sym.to_string(), Span::call_site());
                let rust_type = self.to_rust_type(&x.type_ann.as_ref().unwrap());

                quote! { #name: #rust_type }
            }
            _ => panic!(format!("unhandled {:?}", param)),
        }
    }

    fn to_rust_params<'a, T: Iterator<Item = &'a Param>>(&self, params: T) -> impl ToTokens {
        params.map(|x| self.to_rust_param(x)).collect::<Punctuated<TokenStream, Token![,]>>()
    }

    fn to_rust_fn(&self, fn_decl: &FnDecl) -> TokenStream {
        let name = Ident::new(&fn_decl.ident.sym.to_string(), Span::call_site());
        let return_type = self.to_rust_return_type(&fn_decl.function.return_type);

        let params = self.to_rust_params(fn_decl.function.params.iter());

        quote! { fn #name(#params) #return_type; }
    }

    fn to_rust_class_method_name(&self, method: &ClassMethod) -> String {
        match &method.key {
            PropName::Ident(x) => x.sym.to_string(),
            _ => panic!(),
        }
    }

    fn to_rust_class_member(&self, class: &ClassDecl, member: &ClassMember) -> Option<TokenStream> {
        let class_name = Ident::new(&class.ident.sym.to_string(), Span::call_site());

        match &member {
            ClassMember::Constructor(x) => {
                let params = self.to_rust_params(x.params.iter().map(|x| {
                    if let ParamOrTsParamProp::Param(x) = x {
                        x
                    } else {
                        panic!(format!("unhandled {:?}", x))
                    }
                }));

                Some(quote! {
                    #[wasm_bindgen(constructor)]
                    fn new(#params) -> #class_name;
                })
            }
            ClassMember::ClassProp(x) => {
                if x.accessibility.is_none() || x.accessibility.unwrap() == Accessibility::Public {
                    // TODO We have to use js_sys::Reflect
                    eprintln!("unhandled prop {:?}", member);

                    None
                } else {
                    None
                }
            }
            ClassMember::Method(x) => {
                if x.accessibility.is_none() || x.accessibility.unwrap() == Accessibility::Public {
                    let params = if !x.function.params.is_empty() {
                        let params = self.to_rust_params(x.function.params.iter());

                        quote! {
                            this: &#class_name, #params
                        }
                    } else {
                        quote! {
                            this: &#class_name
                        }
                    };

                    let return_type = self.to_rust_return_type(&x.function.return_type);

                    let mut name = self.to_rust_class_method_name(&x);
                    for member in &class.class.body {
                        if let ClassMember::Method(member_method) = member {
                            if self.to_rust_class_method_name(&member_method) == name && x != member_method {
                                name = format!("{}_{}", name, x.function.params.len())
                            }
                        }
                    }

                    let name = Ident::new(&name, Span::call_site());

                    Some(quote! {
                        #[wasm_bindgen(method)]
                        fn #name(#params) #return_type;
                    })
                } else {
                    None
                }
            }
            _ => panic!(format!("unhandled {:?}", member)),
        }
    }

    fn to_rust_class(&self, class: &ClassDecl) -> TokenStream {
        let name = Ident::new(&class.ident.sym.to_string(), Span::call_site());

        let body = class
            .class
            .body
            .iter()
            .filter_map(|x| self.to_rust_class_member(&class, x))
            .collect::<TokenStream>();

        quote! {
            type #name;

            #body
        }
    }

    fn generate_export_decl(&self, export: &ExportDecl) -> TokenStream {
        match &export.decl {
            Decl::Fn(x) => self.to_rust_fn(x),
            Decl::Class(x) => self.to_rust_class(x),
            _ => panic!(format!("unhandled {:?}", export)),
        }
    }

    fn generate_module_decl(&self, module: &ModuleDecl) -> Option<TokenStream> {
        match &module {
            ModuleDecl::ExportDecl(x) => Some(self.generate_export_decl(x)),
            ModuleDecl::Import(_) => None, // TODO Make an option to handle imports
            _ => panic!(format!("unhandled {:?}", module)),
        }
    }

    fn generate_module_item(&self, item: &ModuleItem) -> Option<TokenStream> {
        match &item {
            ModuleItem::ModuleDecl(x) => self.generate_module_decl(x),
            ModuleItem::Stmt(_) => None,
        }
    }
}

pub fn generate_wasm_bindgen_bindings(content: &str, module_name: &str) -> TokenStream {
    Codegen::generate(content, module_name)
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
        let expected = quote! { fn test(a: JsValue) -> String; };

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

    #[test]
    fn test_class_method() {
        let ts = "export class test {
            constructor(test: string) {}

            test(test: number): string {}

            test1(test: number) {}
        };";
        let expected = quote! {
            type test;

            #[wasm_bindgen(constructor)]
            fn new(test: &str) -> test;

            #[wasm_bindgen(method)]
            fn test(this: &test, test: f64) -> String;

             #[wasm_bindgen(method)]
            fn test1(this: &test, test: f64);
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
            type test;

            #[wasm_bindgen(constructor)]
            fn new(test: &str) -> test;

            #[wasm_bindgen(method)]
            fn test_0(this: &test) -> String;

            #[wasm_bindgen(method)]
            fn test_1(this: &test, test: f64) -> String;

             #[wasm_bindgen(method)]
            fn test_2(this: &test, test: f64, test1: &str);
        };

        assert_codegen_eq!(ts, expected);
    }
}
