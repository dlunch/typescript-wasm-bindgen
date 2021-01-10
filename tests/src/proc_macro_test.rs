use typescript_wasm_bindgen::typescript_wasm_bindgen;
use wasm_bindgen::prelude::wasm_bindgen;

typescript_wasm_bindgen!("tests/ts/test_function.ts", "test");
