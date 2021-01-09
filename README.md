# typescript-wasm-bindgen

WIP, many aspects not yet implemented.

Import typescript definitions directly in your wasm rust app.

## Usage

```typescript
// src/index.ts

export function test(): void {
  console.log("test");
}
```

# build.rs

```rust
// build.rs

use std::path::PathBuf;

use typescript_wasm_bindgen::build_typescript_wasm_binding;

fn main() {
    build_typescript_wasm_binding(&PathBuf::from("./ts/test_function.ts"), "test").unwrap();
}
```

```rust
// lib.rs

use typescript_wasm_bindgen::import_typescript_wasm_binding;
use wasm_bindgen::prelude::wasm_bindgen;

import_typescript_wasm_binding!("test_function");
```

# proc_macro

```rust
// wasm/src/lib.rs

use typescript_wasm_bindgen::typescript;
use wasm_bindgen::prelude::wasm_bindgen;

typescript!("../src/index.ts", "index");

// `typescript!` macro expands like following:
//
// #[wasm_bindgen(module = "index")]
// extern "C" {
//     fn test();
// }
```

## Examples

[simple](https://github.com/dlunch/typescript-wasm-bindgen/tree/main/examples/simple)
