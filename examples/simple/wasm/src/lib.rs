use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(module = "index")]
extern "C" {
    fn test();
}

#[wasm_bindgen(start)]
pub fn main() {
    test();
}
