use std::path::PathBuf;

use typescript_wasm_bindgen::build_typescript_wasm_binding;

fn main() {
    build_typescript_wasm_binding(&PathBuf::from("./ts/test_function.ts"), "test").unwrap();
}
