use typescript_wasm_bindgen::my_macro;

#[test]
fn test() {
    assert_eq!(my_macro!("test"), "TEST")
}