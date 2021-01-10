use std::boxed::Box;
use std::env;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::str;

use typescript_wasm_bindgen_codegen::generate_wasm_bindgen_bindings;

// TODO correct error handling
pub fn build_typescript_wasm_binding(typescript_path: &Path, module_name: &str) -> Result<(), Box<dyn Error>> {
    let content = fs::read(typescript_path)?;
    let content_str = str::from_utf8(&content)?;

    let result = generate_wasm_bindgen_bindings(content_str, module_name);

    let out_dir = &PathBuf::from(env::var("OUT_DIR")?);
    let out_filename = format!("{}.rs", typescript_path.file_stem().unwrap().to_str().unwrap());
    let out_path = out_dir.join(out_filename);

    fs::write(out_path, result.to_string())?;

    Ok(())
}

#[macro_export]
macro_rules! import_typescript_wasm_binding {
    ($filename: expr) => {
        std::include!(concat!(env!("OUT_DIR"), concat!("/", $filename, ".rs")));
    };
}

pub use typescript_wasm_bindgen_macros::typescript_wasm_bindgen;
