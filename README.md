# typescript-wasm-bindgen

WIP, many aspects not yet implemented.

Import typescript directly in your wasm rust app.

## Usage

```typescript
// src/index.ts

export function test(): void {
  console.log("test");
}
```

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
