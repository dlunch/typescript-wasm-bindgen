use typescript_wasm_bindgen::typescript;
use wasm_bindgen::prelude::wasm_bindgen;

typescript!("tests/test.d.ts", "test");

#[test]
fn test() {}
