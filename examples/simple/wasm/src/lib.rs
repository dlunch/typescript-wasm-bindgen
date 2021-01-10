use typescript_wasm_bindgen::typescript_wasm_bindgen;
use wasm_bindgen::prelude::wasm_bindgen;

typescript_wasm_bindgen!("../src/index.ts", "index");

#[wasm_bindgen(start)]
pub fn main() {
    test();
}
