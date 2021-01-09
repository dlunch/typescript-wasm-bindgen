mod proc_macro_test;

use typescript_wasm_bindgen::import_typescript_wasm_binding;
use wasm_bindgen::prelude::wasm_bindgen;

import_typescript_wasm_binding!("test_function");
