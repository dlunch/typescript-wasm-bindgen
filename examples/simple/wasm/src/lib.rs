use typescript_wasm_bindgen::typescript;
use wasm_bindgen::prelude::wasm_bindgen;

typescript!("../src/index.ts", "index");

#[wasm_bindgen(start)]
pub fn main() {
    test();
}
