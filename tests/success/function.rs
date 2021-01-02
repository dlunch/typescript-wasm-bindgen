use typescript_wasm_bindgen::typescript;
use wasm_bindgen::prelude::wasm_bindgen;

typescript!("../../../tests/success/function.ts", "test");

pub fn main() {}
