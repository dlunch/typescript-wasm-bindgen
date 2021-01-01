use typescript_wasm_bindgen::typescript;

#[test]
fn test() {
    assert_eq!(typescript!("tests/test.d.ts"), "TEST")
}
